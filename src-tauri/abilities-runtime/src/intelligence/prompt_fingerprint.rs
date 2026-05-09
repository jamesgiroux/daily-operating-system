//! ADR-0106 prompt fingerprinting boundary.
//!
//! This module owns canonical prompt hashing and provenance fingerprint
//! construction. Callers should not construct `PromptFingerprint` or invoke the
//! low-level hash directly outside this boundary; providers use
//! `replay_fixture_key` for fixture lookup, and abilities use
//! `prompt_fingerprint_from_completion` for provenance.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::abilities::provenance::{
    HashValue, ModelName as ProvenanceModelName, PromptFingerprint, PromptTemplateId,
    PromptVersion,
};

use super::provider::{Completion, FingerprintMetadata, PromptInput};

#[derive(Debug, Clone, Copy)]
pub struct CanonicalPromptRequest<'a> {
    pub prompt: &'a PromptInput,
    pub fingerprint_metadata: &'a FingerprintMetadata,
}

/// ADR-0106 canonical prompt hash shared by provenance and replay lookup.
///
/// The hash is intentionally computed from separated fields, not from a single
/// rendered prompt string: template identity/version, canonicalized template
/// bytes hash, canonical JSON inputs, provider, model, temperature, top_p, and
/// seed. Ad-hoc prompts without template metadata are treated as a synthetic
/// `adhoc` template whose bytes are the rendered prompt text.
#[deprecated(
    note = "provider-boundary only; use replay_fixture_key or prompt_fingerprint_from_completion"
)]
pub fn canonical_prompt_hash(request: CanonicalPromptRequest<'_>) -> String {
    canonical_prompt_hash_impl(request)
}

/// Provider-boundary helper for deterministic replay fixtures.
pub fn replay_fixture_key(prompt: &PromptInput, metadata: &FingerprintMetadata) -> String {
    canonical_prompt_hash_impl(CanonicalPromptRequest {
        prompt,
        fingerprint_metadata: metadata,
    })
}

/// Build the W3-B provenance fingerprint from a provider completion and the
/// exact prompt envelope used for that completion.
pub fn prompt_fingerprint_from_completion(
    completion: &Completion,
    prompt: &PromptInput,
    prompt_template_id: impl Into<String>,
    prompt_template_version: impl Into<String>,
) -> PromptFingerprint {
    let meta = &completion.fingerprint_metadata;
    let canonical_hash = replay_fixture_key(prompt, meta);
    PromptFingerprint {
        provider: meta.provider.as_str().to_string(),
        model: ProvenanceModelName(meta.model.as_str().to_string()),
        prompt_template_id: PromptTemplateId(prompt_template_id.into()),
        prompt_template_version: PromptVersion(prompt_template_version.into()),
        canonical_prompt_hash: HashValue::new(canonical_hash),
        temperature: meta.temperature,
        top_p: meta.top_p,
        seed: meta.seed,
        tokens_input: meta.tokens_input,
        tokens_output: meta.tokens_output,
        provider_completion_id: meta.provider_completion_id.clone(),
    }
}

pub fn canonical_template_hash(template_bytes: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(canonical_prompt_text(template_bytes).as_bytes());
    hex::encode(hasher.finalize())
}

pub fn canonicalize_json_value(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_json_value).collect()),
        Value::Object(object) => {
            let sorted = object
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize_json_value(value)))
                .collect::<BTreeMap<_, _>>();
            Value::Object(sorted.into_iter().collect())
        }
        other => other.clone(),
    }
}

fn canonical_prompt_hash_impl(request: CanonicalPromptRequest<'_>) -> String {
    use sha2::{Digest, Sha256};

    let canonical = canonical_prompt_request_value(request);
    let canonical = canonical_json_string(&canonical);
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

fn canonical_prompt_request_value(request: CanonicalPromptRequest<'_>) -> Value {
    let prompt = request.prompt;
    let meta = request.fingerprint_metadata;
    let template_hash = prompt
        .template_hash
        .clone()
        .unwrap_or_else(|| canonical_template_hash(&prompt.text));
    let inputs = prompt
        .canonical_json_inputs
        .as_ref()
        .map(canonicalize_json_value)
        .unwrap_or(Value::Null);

    let mut object = Map::new();
    object.insert(
        "schema".to_string(),
        Value::String("adr-0106-canonical-prompt-v1".to_string()),
    );
    object.insert(
        "template_id".to_string(),
        Value::String(
            prompt
                .template_id
                .clone()
                .unwrap_or_else(|| "adhoc".to_string()),
        ),
    );
    object.insert(
        "template_version".to_string(),
        Value::String(
            prompt
                .template_version
                .clone()
                .unwrap_or_else(|| "unversioned".to_string()),
        ),
    );
    object.insert("template_hash".to_string(), Value::String(template_hash));
    object.insert("canonical_json_inputs".to_string(), inputs);
    object.insert(
        "provider".to_string(),
        Value::String(meta.provider.as_str().to_string()),
    );
    object.insert(
        "model".to_string(),
        Value::String(meta.model.as_str().to_string()),
    );
    object.insert(
        "temperature".to_string(),
        Value::String(canonical_f32_hex(meta.temperature)),
    );
    object.insert(
        "top_p".to_string(),
        meta.top_p
            .map(canonical_f32_hex)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    object.insert(
        "seed".to_string(),
        meta.seed.map(Value::from).unwrap_or(Value::Null),
    );

    Value::Object(object)
}

fn canonical_prompt_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = normalized
        .split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>();

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    let mut canonical = lines.join("\n");
    canonical.push('\n');
    canonical
}

fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonicalize_json_value(value))
        .unwrap_or_else(|error| format!("{{\"canonicalization_error\":\"{error}\"}}"))
}

fn canonical_f32(value: f32) -> [u8; 4] {
    value.to_be_bytes()
}

fn canonical_f32_hex(value: f32) -> String {
    hex::encode(canonical_f32(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::provider::{ModelName, ProviderKind};

    fn metadata() -> FingerprintMetadata {
        FingerprintMetadata {
            provider: ProviderKind::ClaudeCode,
            model: ModelName::new("claude-test"),
            temperature: 1.0,
            top_p: Some(0.9),
            seed: Some(7),
            tokens_input: None,
            tokens_output: None,
            provider_completion_id: None,
        }
    }

    #[test]
    fn canonical_prompt_hash_is_stable_for_same_text() {
        let a = PromptInput::new("same prompt");
        let b = PromptInput::new("same prompt");
        let meta = FingerprintMetadata::default();
        assert_eq!(replay_fixture_key(&a, &meta), replay_fixture_key(&b, &meta));
    }

    #[test]
    fn canonical_prompt_hash_distinguishes_adr_0106_fields() {
        let template_hash = canonical_template_hash("Hello {{name}}\n");
        let a = PromptInput::new("Hello Ada")
            .with_template("greeting", "1.0.0", template_hash.clone())
            .with_canonical_json_inputs(serde_json::json!({"name": "Ada"}));
        let b = PromptInput::new("Hello Ada")
            .with_template("greeting", "1.0.1", template_hash)
            .with_canonical_json_inputs(serde_json::json!({"name": "Ada"}));
        let meta = metadata();
        assert_ne!(replay_fixture_key(&a, &meta), replay_fixture_key(&b, &meta));
    }

    #[test]
    fn canonical_prompt_hash_normalizes_line_endings_and_trailing_whitespace() {
        let a = PromptInput::new("Hello Ada\r\nTrailing spaces   \r\n");
        let b = PromptInput::new("Hello Ada\nTrailing spaces\n\n");
        let meta = FingerprintMetadata::default();
        assert_eq!(replay_fixture_key(&a, &meta), replay_fixture_key(&b, &meta));
    }

    #[test]
    fn canonical_prompt_hash_sorts_json_object_inputs() {
        let template_hash = canonical_template_hash("Hello {{json}}\n");
        let a = PromptInput::new("Hello")
            .with_template("json", "1.0.0", template_hash.clone())
            .with_canonical_json_inputs(serde_json::json!({"b": 2, "a": {"d": 4, "c": 3}}));
        let b = PromptInput::new("Hello")
            .with_template("json", "1.0.0", template_hash)
            .with_canonical_json_inputs(serde_json::json!({"a": {"c": 3, "d": 4}, "b": 2}));
        let meta = metadata();
        assert_eq!(replay_fixture_key(&a, &meta), replay_fixture_key(&b, &meta));
    }

    #[test]
    fn canonical_f32_uses_adr_0106_big_endian_ieee754_bytes() {
        assert_eq!(canonical_f32(0.0), [0x00, 0x00, 0x00, 0x00]);
        assert_eq!(canonical_f32(0.5), [0x3F, 0x00, 0x00, 0x00]);
        assert_eq!(canonical_f32(0.9), [0x3F, 0x66, 0x66, 0x66]);
        assert_eq!(canonical_f32(1.0), [0x3F, 0x80, 0x00, 0x00]);
    }

    #[test]
    fn canonical_prompt_request_pins_sampling_hex_fields() {
        let prompt = PromptInput::new("Sampling stable");
        let mut meta = metadata();
        meta.seed = None;

        for (temperature, expected_hex) in [
            (0.0, "00000000"),
            (0.5, "3f000000"),
            (1.0, "3f800000"),
        ] {
            meta.temperature = temperature;
            meta.top_p = None;
            let canonical = canonical_prompt_request_value(CanonicalPromptRequest {
                prompt: &prompt,
                fingerprint_metadata: &meta,
            });
            assert_eq!(
                canonical["temperature"], expected_hex,
                "temperature={temperature}"
            );
        }

        meta.temperature = 0.0;
        for (top_p, expected_hex) in [(0.9, "3f666666"), (1.0, "3f800000")] {
            meta.top_p = Some(top_p);
            let canonical = canonical_prompt_request_value(CanonicalPromptRequest {
                prompt: &prompt,
                fingerprint_metadata: &meta,
            });
            assert_eq!(canonical["top_p"], expected_hex, "top_p={top_p}");
        }
    }

    #[test]
    fn canonical_prompt_hash_has_golden_temperature_hashes() {
        let template_hash = canonical_template_hash("Temperature {{value}}\n");
        let prompt = PromptInput::new("Temperature stable")
            .with_template("temperature-golden", "1.0.0", template_hash)
            .with_canonical_json_inputs(serde_json::json!({"value": "stable"}));
        let mut meta = metadata();
        meta.top_p = None;
        meta.seed = None;

        let actual = [0.0, 0.5, 1.0]
            .into_iter()
            .map(|temperature| {
                meta.temperature = temperature;
                replay_fixture_key(&prompt, &meta)
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                "cdb5732416f89d9b71fcddb44a8e366996d95d9ba6114b70e645ea6cb58f72ae",
                "4e8e73e18b38bf08513663b27c19f015797c267f3a18107256da61af972d7475",
                "a9b77113ef631732951f73be46ff8384fd4465966e04c1a87b51679c8235b2c7"
            ]
        );
    }

    #[test]
    fn prompt_fingerprint_from_completion_carries_provider_metadata() {
        let prompt = PromptInput::new("Hello")
            .with_template("greeting", "1.0.0", canonical_template_hash("Hello\n"));
        let completion = Completion {
            text: "response".to_string(),
            fingerprint_metadata: metadata(),
        };

        let fingerprint =
            prompt_fingerprint_from_completion(&completion, &prompt, "greeting", "1.0.0");

        assert_eq!(fingerprint.provider, "claude_code");
        assert_eq!(fingerprint.model.0, "claude-test");
        assert_eq!(fingerprint.prompt_template_id.0, "greeting");
        assert_eq!(fingerprint.prompt_template_version.0, "1.0.0");
        assert_eq!(fingerprint.temperature, 1.0);
        assert_eq!(fingerprint.top_p, Some(0.9));
        assert_eq!(fingerprint.seed, Some(7));
    }
}
