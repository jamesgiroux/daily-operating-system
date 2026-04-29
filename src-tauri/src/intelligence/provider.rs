//! DOS-259 (W2-B): `IntelligenceProvider` trait + supporting types.
//!
//! Combines ADR-0106 §3 (`Completion` / `FingerprintMetadata` / trait shape)
//! and ADR-0091 (`Send + Sync`, `ProviderError`, AppState-owned `Arc`).
//!
//! Two production implementations land in W2-B:
//! - `pty_provider::PtyClaudeCode` — wraps `pty::PtyManager` for local Claude Code calls.
//! - `glean_provider::GleanIntelligenceProvider` — wraps the Glean MCP `chat` tool.
//!
//! `ReplayProvider` lives in this module gated for `#[cfg(test)]` and the
//! `Evaluate` execution mode. Fixture file format and on-disk layout are
//! out of scope here — W4-B (DOS-216) owns that.
//!
//! ## Provider seam routing (per ADR-0104 + L6 2026-04-29 ruling)
//!
//! `select_provider(ctx: &AbilityContext, tier)` is the single source of
//! ability-context provider selection. `AbilityContext` lands in W3-A
//! (DOS-210 ability registry); until then early callers — `intel_queue.rs`
//! and `services::intelligence` — read `AppState`'s configured provider
//! `Arc` per ADR-0091 ("read at call time; switch mid-queue takes effect
//! on next dequeue"). When `AbilityContext` lands those callers migrate
//! to `select_provider(ability_ctx, tier)`.

use std::sync::Arc;

use async_trait::async_trait;

pub use crate::pty::ModelTier;

/// Provider taxonomy per ADR-0106 §3.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    ClaudeCode,
    Ollama,
    OpenAI,
    Other(&'static str),
}

impl ProviderKind {
    pub fn as_str(&self) -> &str {
        match self {
            ProviderKind::ClaudeCode => "claude_code",
            ProviderKind::Ollama => "ollama",
            ProviderKind::OpenAI => "openai",
            ProviderKind::Other(s) => s,
        }
    }
}

/// Newtype model identifier per ADR-0106 — opaque string the provider
/// resolves against its own configuration.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelName(pub String);

impl ModelName {
    pub fn new(s: impl Into<String>) -> Self {
        ModelName(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ModelName {
    fn from(s: String) -> Self {
        ModelName(s)
    }
}

impl From<&str> for ModelName {
    fn from(s: &str) -> Self {
        ModelName(s.to_string())
    }
}

/// Provider-known fingerprint fields per ADR-0106 §3.
///
/// `tokens_input`/`tokens_output` are optional — PTY providers do not
/// report token counts, Glean does not either today. DOS-213 lands the
/// canonical hash; W2-B carries only the fields a provider knows at
/// `complete()` time.
#[derive(Debug, Clone, Default)]
pub struct FingerprintMetadata {
    pub provider: Option<ProviderKind>,
    pub model: Option<ModelName>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub seed: Option<u64>,
    pub tokens_input: Option<u32>,
    pub tokens_output: Option<u32>,
    pub provider_completion_id: Option<String>,
}

/// Provider response.
#[derive(Debug, Clone, Default)]
pub struct Completion {
    pub text: String,
    pub fingerprint_metadata: FingerprintMetadata,
}

/// Prompt envelope passed to `complete()`.
///
/// `text` is the rendered prompt the provider executes. `workspace` is
/// the optional working directory for PTY-style providers (Claude Code
/// requires a workspace; HTTP-style providers ignore it).
/// `template_id` and `template_hash` are forward-looking hooks DOS-213
/// will populate when production prompt fingerprinting lands; W2-B
/// callers may leave them `None`.
#[derive(Debug, Clone, Default)]
pub struct PromptInput {
    pub text: String,
    pub workspace: Option<std::path::PathBuf>,
    pub template_id: Option<String>,
    pub template_hash: Option<String>,
}

impl PromptInput {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            workspace: None,
            template_id: None,
            template_hash: None,
        }
    }

    pub fn with_workspace(mut self, ws: impl Into<std::path::PathBuf>) -> Self {
        self.workspace = Some(ws.into());
        self
    }
}

/// Provider error surface per ADR-0091.
///
/// Variants cover the three failure modes downstream callers branch on:
/// transient (retryable), permanent (configuration / auth), and
/// mode-routing (Simulate/Evaluate fail-closed).
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// Transient failure — caller may retry or fall back to another provider.
    #[error("provider transient failure: {0}")]
    Transient(String),

    /// Permanent failure — auth, configuration, or unrecoverable upstream error.
    #[error("provider permanent failure: {0}")]
    Permanent(String),

    /// Operation timed out at the provider layer.
    #[error("provider timed out after {seconds}s")]
    Timeout { seconds: u64 },

    /// Provider rejected the prompt (parse / length / policy).
    #[error("provider rejected prompt: {0}")]
    InvalidPrompt(String),

    /// Replay fixture did not contain a matching completion.
    /// Used by `ReplayProvider` in `Evaluate` mode; never falls through to live.
    #[error("replay fixture missing for prompt hash {0}")]
    ReplayFixtureMissing(String),

    /// Mode routing rejected the call (e.g., `Simulate` invoked a generative path).
    #[error("provider not supported in current execution mode")]
    ModeNotSupported,
}

/// Core provider trait. Send + Sync so `Arc<dyn IntelligenceProvider>`
/// can move across tasks freely (per ADR-0091).
#[async_trait]
pub trait IntelligenceProvider: Send + Sync {
    /// Run a completion at the given tier.
    async fn complete(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError>;

    /// Report which `ProviderKind` this implementation is.
    fn provider_kind(&self) -> ProviderKind;

    /// Report the model name that would be used for the given tier.
    fn current_model(&self, tier: ModelTier) -> ModelName;
}

/// Stable hash of a prompt's text used for `ReplayProvider` lookups.
///
/// W2-B uses a SHA-256 of the rendered prompt text. DOS-213 lands a
/// canonical-prompt hash that includes template id + parameter values
/// for production fingerprinting; this is the W2-B-only hook.
pub fn prompt_replay_hash(prompt: &PromptInput) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(prompt.text.as_bytes());
    if let Some(ref id) = prompt.template_id {
        hasher.update(b"\0template_id=");
        hasher.update(id.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// In-memory replay provider for tests + `Evaluate` mode.
///
/// Stores `(hash, completion)` pairs supplied at construction. On
/// `complete()`, hashes the incoming `PromptInput.text` and looks it up;
/// returns `ProviderError::ReplayFixtureMissing` if absent — never falls
/// through to a live path.
pub struct ReplayProvider {
    fixtures: std::collections::HashMap<String, Completion>,
    provider_kind: ProviderKind,
    model_for_tier: std::collections::HashMap<ModelTier, ModelName>,
}

impl ReplayProvider {
    /// Build a replay provider from a `(hash → completion)` map.
    pub fn new(fixtures: std::collections::HashMap<String, Completion>) -> Self {
        Self {
            fixtures,
            provider_kind: ProviderKind::Other("replay"),
            model_for_tier: std::collections::HashMap::new(),
        }
    }

    /// Convenience: build a replay provider from `(prompt_text → text)` pairs,
    /// hashing each prompt with `prompt_replay_hash`.
    pub fn from_prompt_pairs<I, P, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (P, T)>,
        P: Into<String>,
        T: Into<String>,
    {
        let mut fixtures = std::collections::HashMap::new();
        for (prompt_text, completion_text) in pairs {
            let p = PromptInput::new(prompt_text);
            let key = prompt_replay_hash(&p);
            fixtures.insert(
                key,
                Completion {
                    text: completion_text.into(),
                    fingerprint_metadata: FingerprintMetadata {
                        provider: Some(ProviderKind::Other("replay")),
                        ..Default::default()
                    },
                },
            );
        }
        Self::new(fixtures)
    }

    pub fn with_provider_kind(mut self, kind: ProviderKind) -> Self {
        self.provider_kind = kind;
        self
    }

    pub fn with_model_for_tier(mut self, tier: ModelTier, model: ModelName) -> Self {
        self.model_for_tier.insert(tier, model);
        self
    }
}

#[async_trait]
impl IntelligenceProvider for ReplayProvider {
    async fn complete(
        &self,
        prompt: PromptInput,
        _tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        let key = prompt_replay_hash(&prompt);
        self.fixtures
            .get(&key)
            .cloned()
            .ok_or(ProviderError::ReplayFixtureMissing(key))
    }

    fn provider_kind(&self) -> ProviderKind {
        self.provider_kind.clone()
    }

    fn current_model(&self, tier: ModelTier) -> ModelName {
        self.model_for_tier
            .get(&tier)
            .cloned()
            .unwrap_or_else(|| ModelName::new("replay"))
    }
}

/// Forward-looking `select_provider` hook for ability-execution contexts.
///
/// `AbilityContext` lands in W3-A (DOS-210 ability registry). Until then
/// this signature exists in source but is unreferenced — early callers
/// route via `AppState`'s configured provider `Arc` per ADR-0091.
///
/// The L6 2026-04-29 ruling pinned this signature on `&AbilityContext`,
/// not `&ServiceContext` (per ADR-0104 split: provider lives on
/// `AbilityContext`, mode-routing is the only thing the factory reads
/// from `ServiceContext`-adjacent context).
///
/// Stub returns `Err(ProviderError::ModeNotSupported)` — calling this
/// before W3-A is a programming error.
#[allow(dead_code)]
pub fn select_provider_stub(_tier: ModelTier) -> Result<Arc<dyn IntelligenceProvider>, ProviderError>
{
    Err(ProviderError::ModeNotSupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_completion(text: &str) -> Completion {
        Completion {
            text: text.to_string(),
            fingerprint_metadata: FingerprintMetadata {
                provider: Some(ProviderKind::Other("replay")),
                model: Some(ModelName::new("test-model")),
                ..Default::default()
            },
        }
    }

    #[tokio::test]
    async fn replay_provider_returns_canned_completion() {
        let provider = ReplayProvider::from_prompt_pairs([("hello world", "canned response")]);
        let prompt = PromptInput::new("hello world");
        let got = provider
            .complete(prompt, ModelTier::Synthesis)
            .await
            .expect("replay returns canned completion");
        assert_eq!(got.text, "canned response");
        assert_eq!(
            got.fingerprint_metadata.provider,
            Some(ProviderKind::Other("replay"))
        );
    }

    #[tokio::test]
    async fn replay_provider_fixture_miss_returns_replay_fixture_missing() {
        let provider = ReplayProvider::from_prompt_pairs([("known", "ok")]);
        let prompt = PromptInput::new("unknown prompt");
        let err = provider
            .complete(prompt, ModelTier::Synthesis)
            .await
            .expect_err("missing fixture must error");
        match err {
            ProviderError::ReplayFixtureMissing(_) => (),
            other => panic!("expected ReplayFixtureMissing, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn replay_provider_does_not_fall_through_to_live() {
        // ADR-0104 invariant: Evaluate-mode replay routing must structurally
        // refuse to invoke any live path. Modeled by ReplayProvider returning
        // a typed error rather than ever calling network/PTY.
        let provider = ReplayProvider::new(std::collections::HashMap::new());
        let prompt = PromptInput::new("anything");
        let err = provider
            .complete(prompt, ModelTier::Synthesis)
            .await
            .expect_err("empty replay must always error");
        assert!(matches!(err, ProviderError::ReplayFixtureMissing(_)));
    }

    #[tokio::test]
    async fn replay_provider_concurrent_invocations_all_succeed() {
        // Proves the Send + Sync invariant from ADR-0091 by driving N
        // simultaneous .complete() calls against a fixture-backed provider.
        let mut fixtures = std::collections::HashMap::new();
        for i in 0..32u32 {
            let p = PromptInput::new(format!("prompt-{i}"));
            fixtures.insert(prompt_replay_hash(&p), fixture_completion(&format!("r-{i}")));
        }
        let provider: Arc<dyn IntelligenceProvider> = Arc::new(ReplayProvider::new(fixtures));
        let mut handles = Vec::new();
        for i in 0..32u32 {
            let p = Arc::clone(&provider);
            handles.push(tokio::spawn(async move {
                let prompt = PromptInput::new(format!("prompt-{i}"));
                p.complete(prompt, ModelTier::Synthesis).await
            }));
        }
        for (i, h) in handles.into_iter().enumerate() {
            let got = h.await.expect("task join").expect("complete ok");
            assert_eq!(got.text, format!("r-{i}"));
        }
    }

    #[test]
    fn prompt_replay_hash_is_stable_for_same_text() {
        let a = PromptInput::new("same prompt");
        let b = PromptInput::new("same prompt");
        assert_eq!(prompt_replay_hash(&a), prompt_replay_hash(&b));
    }

    #[test]
    fn prompt_replay_hash_distinguishes_template_id() {
        let mut a = PromptInput::new("text");
        let mut b = PromptInput::new("text");
        a.template_id = Some("v1".to_string());
        b.template_id = Some("v2".to_string());
        assert_ne!(prompt_replay_hash(&a), prompt_replay_hash(&b));
    }

    #[test]
    fn provider_kind_as_str_is_stable() {
        assert_eq!(ProviderKind::ClaudeCode.as_str(), "claude_code");
        assert_eq!(ProviderKind::Other("glean").as_str(), "glean");
    }

    #[test]
    fn select_provider_stub_returns_mode_not_supported_until_w3a() {
        match select_provider_stub(ModelTier::Synthesis) {
            Err(ProviderError::ModeNotSupported) => {}
            Err(other) => panic!("expected ModeNotSupported, got {other}"),
            Ok(_) => panic!("stub must error until W3-A wires AbilityContext"),
        }
    }
}
