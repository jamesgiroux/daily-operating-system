pub use abilities_runtime::intelligence::provider::*;

use std::{fmt, time::Duration};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const HTTP_COMPLETION_TIMEOUT_SECS: u64 = 240;
const DEFAULT_HTTP_TEMPERATURE: f32 = 0.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderModelConfig {
    pub synthesis: ModelName,
    pub extraction: ModelName,
    pub background: ModelName,
    pub mechanical: ModelName,
}

impl ProviderModelConfig {
    pub fn ollama_defaults() -> Self {
        Self {
            synthesis: ModelName::new("llama3.3:70b"),
            extraction: ModelName::new("llama3.2:3b"),
            background: ModelName::new("llama3.2:3b"),
            mechanical: ModelName::new("llama3.2:3b"),
        }
    }

    pub fn openai_defaults() -> Self {
        Self {
            synthesis: ModelName::new("gpt-4o"),
            extraction: ModelName::new("gpt-4o-mini"),
            background: ModelName::new("gpt-4o-mini"),
            mechanical: ModelName::new("gpt-4o-mini"),
        }
    }

    fn model_for_tier(&self, tier: ModelTier) -> ModelName {
        match tier {
            ModelTier::Synthesis => self.synthesis.clone(),
            ModelTier::Extraction => self.extraction.clone(),
            ModelTier::Background => self.background.clone(),
            ModelTier::Mechanical => self.mechanical.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    inner: ChatCompletionProvider,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self::with_config(
            "http://localhost:11434/v1",
            ProviderModelConfig::ollama_defaults(),
            DEFAULT_HTTP_TEMPERATURE,
            None,
            None,
        )
    }

    pub fn with_config(
        base_url: impl Into<String>,
        models: ProviderModelConfig,
        temperature: f32,
        top_p: Option<f32>,
        seed: Option<u64>,
    ) -> Self {
        Self {
            inner: ChatCompletionProvider::new(
                ProviderKind::Ollama,
                base_url,
                None,
                models,
                temperature,
                top_p,
                seed,
            ),
        }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IntelligenceProvider for OllamaProvider {
    async fn complete(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        self.inner.complete(prompt, tier).await
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    fn current_model(&self, tier: ModelTier) -> ModelName {
        self.inner.current_model(tier)
    }
}

#[derive(Clone)]
pub struct OpenAIProvider {
    inner: ChatCompletionProvider,
}

impl OpenAIProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_config(
            "https://api.openai.com/v1",
            api_key,
            ProviderModelConfig::openai_defaults(),
            DEFAULT_HTTP_TEMPERATURE,
            None,
            None,
        )
    }

    pub fn with_config(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        models: ProviderModelConfig,
        temperature: f32,
        top_p: Option<f32>,
        seed: Option<u64>,
    ) -> Self {
        Self {
            inner: ChatCompletionProvider::new(
                ProviderKind::OpenAI,
                base_url,
                Some(api_key.into()),
                models,
                temperature,
                top_p,
                seed,
            ),
        }
    }
}

impl fmt::Debug for OpenAIProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAIProvider")
            .field("inner", &self.inner)
            .finish()
    }
}

#[async_trait]
impl IntelligenceProvider for OpenAIProvider {
    async fn complete(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        self.inner.complete(prompt, tier).await
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::OpenAI
    }

    fn current_model(&self, tier: ModelTier) -> ModelName {
        self.inner.current_model(tier)
    }
}

#[derive(Clone)]
struct ChatCompletionProvider {
    kind: ProviderKind,
    base_url: String,
    api_key: Option<String>,
    models: ProviderModelConfig,
    temperature: f32,
    top_p: Option<f32>,
    seed: Option<u64>,
    client: reqwest::Client,
}

struct RedactedApiKey<'a>(&'a Option<String>);

impl fmt::Debug for RedactedApiKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(_) => f.write_str("Some(**REDACTED**)"),
            None => f.write_str("None"),
        }
    }
}

impl fmt::Debug for ChatCompletionProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChatCompletionProvider")
            .field("kind", &self.kind)
            .field("base_url", &self.base_url)
            .field("api_key", &RedactedApiKey(&self.api_key))
            .field("models", &self.models)
            .field("temperature", &self.temperature)
            .field("top_p", &self.top_p)
            .field("seed", &self.seed)
            .field("client", &self.client)
            .finish()
    }
}

impl ChatCompletionProvider {
    fn new(
        kind: ProviderKind,
        base_url: impl Into<String>,
        api_key: Option<String>,
        models: ProviderModelConfig,
        temperature: f32,
        top_p: Option<f32>,
        seed: Option<u64>,
    ) -> Self {
        Self {
            kind,
            base_url: base_url.into(),
            api_key,
            models,
            temperature,
            top_p,
            seed,
            client: reqwest::Client::new(),
        }
    }

    async fn complete(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        let model = self.current_model(tier);
        let body = ChatCompletionRequest {
            model: model.as_str(),
            messages: vec![ChatMessage {
                role: "user",
                content: &prompt.text,
            }],
            temperature: self.temperature,
            top_p: self.top_p,
            seed: self.seed,
            stream: false,
        };

        let mut request = self.client.post(self.chat_completions_url()).json(&body);
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }

        let (status, response_body) =
            tokio::time::timeout(Duration::from_secs(HTTP_COMPLETION_TIMEOUT_SECS), async {
                let response = request.send().await.map_err(provider_error_from_reqwest)?;
                let status = response.status();
                let response_body = response.text().await.map_err(provider_error_from_reqwest)?;
                Ok::<_, ProviderError>((status, response_body))
            })
            .await
            .map_err(|_| ProviderError::Timeout {
                seconds: HTTP_COMPLETION_TIMEOUT_SECS,
            })??;

        if !status.is_success() {
            return Err(provider_error_from_status(status, response_body, tier));
        }

        completion_from_response_body(
            &response_body,
            FingerprintMetadata {
                provider: self.kind.clone(),
                model,
                temperature: self.temperature,
                top_p: self.top_p,
                seed: self.seed,
                tokens_input: None,
                tokens_output: None,
                provider_completion_id: None,
            },
        )
    }

    fn current_model(&self, tier: ModelTier) -> ModelName {
        self.models.model_for_tier(tier)
    }

    fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    id: Option<String>,
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
}

fn completion_from_response_body(
    response_body: &str,
    mut metadata: FingerprintMetadata,
) -> Result<Completion, ProviderError> {
    let parsed = serde_json::from_str::<ChatCompletionResponse>(response_body)
        .map_err(|error| ProviderError::MalformedResponse(error.to_string()))?;
    let text = parsed
        .choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .ok_or_else(|| {
            ProviderError::MalformedResponse("chat completion response had no content".to_string())
        })?;

    if let Some(usage) = parsed.usage {
        metadata.tokens_input = usage.prompt_tokens;
        metadata.tokens_output = usage.completion_tokens;
    }
    metadata.provider_completion_id = parsed.id;

    Ok(Completion {
        text,
        fingerprint_metadata: metadata,
    })
}

fn provider_error_from_reqwest(error: reqwest::Error) -> ProviderError {
    if error.is_timeout() {
        ProviderError::Timeout {
            seconds: HTTP_COMPLETION_TIMEOUT_SECS,
        }
    } else if error.is_connect() {
        ProviderError::Unavailable(error.to_string())
    } else {
        ProviderError::Transient(error.to_string())
    }
}

fn provider_error_from_status(
    status: reqwest::StatusCode,
    body: String,
    tier: ModelTier,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => ProviderError::Permanent(body),
        404 => ProviderError::TierUnavailable {
            tier,
            message: body,
        },
        408 | 429 | 500..=599 => ProviderError::Transient(body),
        413 => ProviderError::PromptTooLarge {
            tokens: 0,
            limit: 0,
        },
        _ => ProviderError::InvalidPrompt(body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(provider: ProviderKind) -> FingerprintMetadata {
        FingerprintMetadata {
            provider,
            model: ModelName::new("test-model"),
            temperature: 0.2,
            top_p: Some(0.95),
            seed: Some(217),
            tokens_input: None,
            tokens_output: None,
            provider_completion_id: None,
        }
    }

    #[test]
    fn openai_provider_reports_tier_models() {
        let provider = OpenAIProvider::new("test-key");
        assert_eq!(provider.provider_kind(), ProviderKind::OpenAI);
        assert_eq!(
            provider.current_model(ModelTier::Synthesis).as_str(),
            "gpt-4o"
        );
        assert_eq!(
            provider.current_model(ModelTier::Extraction).as_str(),
            "gpt-4o-mini"
        );
    }

    #[test]
    fn ollama_provider_reports_tier_models() {
        let provider = OllamaProvider::new();
        assert_eq!(provider.provider_kind(), ProviderKind::Ollama);
        assert_eq!(
            provider.current_model(ModelTier::Synthesis).as_str(),
            "llama3.3:70b"
        );
        assert_eq!(
            provider.current_model(ModelTier::Mechanical).as_str(),
            "llama3.2:3b"
        );
    }

    #[test]
    fn provider_debug_redacts_api_key() {
        let api_key = "sk-test-real-secret-value";
        let inner = ChatCompletionProvider::new(
            ProviderKind::OpenAI,
            "https://api.openai.com/v1",
            Some(api_key.to_string()),
            ProviderModelConfig::openai_defaults(),
            DEFAULT_HTTP_TEMPERATURE,
            None,
            None,
        );
        let inner_debug = format!("{inner:?}");
        assert!(!inner_debug.contains(api_key));
        assert!(inner_debug.contains("api_key: Some(**REDACTED**)"));

        let provider = OpenAIProvider { inner };

        let provider_debug = format!("{provider:?}");
        assert!(!provider_debug.contains(api_key));
        assert!(provider_debug.contains("api_key: Some(**REDACTED**)"));

        let none_debug = format!("{:?}", OllamaProvider::new());
        assert!(none_debug.contains("api_key: None"));
    }

    #[test]
    fn chat_response_parsing_preserves_fingerprint_metadata() {
        let body = r#"{
            "id": "cmpl_test",
            "choices": [{"message": {"content": "hello"}}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 7}
        }"#;

        let completion = completion_from_response_body(body, metadata(ProviderKind::OpenAI))
            .expect("parse completion");

        assert_eq!(completion.text, "hello");
        assert_eq!(
            completion.fingerprint_metadata.provider,
            ProviderKind::OpenAI
        );
        assert_eq!(completion.fingerprint_metadata.tokens_input, Some(5));
        assert_eq!(completion.fingerprint_metadata.tokens_output, Some(7));
        assert_eq!(
            completion
                .fingerprint_metadata
                .provider_completion_id
                .as_deref(),
            Some("cmpl_test")
        );
    }
}
