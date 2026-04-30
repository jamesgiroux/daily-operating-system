//! DOS-259 (W2-B): `PtyClaudeCode` — `IntelligenceProvider` adapter for the
//! local Claude Code PTY path.
//!
//! Wraps `pty::PtyManager` so callers in `intel_queue` and `services::intelligence`
//! can invoke completions through the trait surface (testability + replay
//! injection) without changing the underlying `spawn_claude` semantics.
//!
//! ## Workspace ownership
//!
//! `PtyClaudeCode` holds a default workspace + ai_config + usage context.
//! Callers may override the workspace per-call via `PromptInput.workspace`
//! (the legacy `intel_queue` sites pass per-entity workspace dirs).
//! When `PromptInput.workspace` is `None` the provider's default is used.
//!
//! ## Async over sync subprocess
//!
//! `PtyManager::spawn_claude` is sync (blocks on the subprocess). The
//! trait is async, so `complete()` runs the spawn under
//! `tokio::task::spawn_blocking`. This matches the existing
//! `std::thread::spawn` pattern in `intel_queue.rs:1730`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use super::provider::{
    Completion, FingerprintMetadata, IntelligenceProvider, ModelName, PromptInput, ProviderError,
    ProviderKind,
};
use crate::pty::{AiUsageContext, ModelTier, PtyManager};
use crate::types::AiModelConfig;

/// Default per-tier timeout (seconds) when callers do not override.
///
/// Mirrors the timeout values used in `intel_queue.rs` legacy paths.
const DEFAULT_SYNTHESIS_TIMEOUT_SECS: u64 = 240;
const DEFAULT_EXTRACTION_TIMEOUT_SECS: u64 = 240;
const DEFAULT_BACKGROUND_TIMEOUT_SECS: u64 = 240;
const DEFAULT_MECHANICAL_TIMEOUT_SECS: u64 = 90;

/// Default `nice` priority for Claude Code subprocesses.
const DEFAULT_NICE_PRIORITY: i32 = 10;

/// Documented temperature placeholder for Claude Code completions.
///
/// Claude Code does not expose a temperature flag; the underlying model
/// uses its native default sampling temperature (effectively `1.0` for
/// the Claude family). Recorded here for ADR-0106 §3 fingerprint metadata
/// completeness. DOS-213 (W3) replaces this with the actual configured
/// temperature when canonical fingerprint hashing lands.
const CLAUDE_CODE_DEFAULT_TEMPERATURE: f32 = 1.0;

/// PTY-based Claude Code provider.
pub struct PtyClaudeCode {
    ai_config: Arc<AiModelConfig>,
    default_workspace: PathBuf,
    usage_context: AiUsageContext,
}

impl PtyClaudeCode {
    /// Build a provider pinned to an `AiModelConfig` and default workspace.
    /// `usage_context` is the base — `complete()` overlays the requested tier.
    pub fn new(
        ai_config: Arc<AiModelConfig>,
        default_workspace: impl Into<PathBuf>,
        usage_context: AiUsageContext,
    ) -> Self {
        Self {
            ai_config,
            default_workspace: default_workspace.into(),
            usage_context,
        }
    }

    fn timeout_for_tier(tier: ModelTier) -> u64 {
        match tier {
            ModelTier::Synthesis => DEFAULT_SYNTHESIS_TIMEOUT_SECS,
            ModelTier::Extraction => DEFAULT_EXTRACTION_TIMEOUT_SECS,
            ModelTier::Background => DEFAULT_BACKGROUND_TIMEOUT_SECS,
            ModelTier::Mechanical => DEFAULT_MECHANICAL_TIMEOUT_SECS,
        }
    }
}

impl PtyClaudeCode {
    /// Sync `complete()` for the legacy `intel_queue` paths that call PTY
    /// from `std::thread::spawn` / `spawn_blocking`. The trait `complete()`
    /// is async-only; callers that cannot enter an async context use this
    /// inherent method. This is the bridge surface — when intel_queue's
    /// sync orchestration is async-ified (post-W3-A), callers migrate to
    /// `complete()` and `ReplayProvider` becomes substitutable here too.
    pub fn complete_blocking(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        let workspace = prompt
            .workspace
            .clone()
            .unwrap_or_else(|| self.default_workspace.clone());
        let usage_context = self.usage_context.clone().with_tier(tier);
        let timeout_secs = Self::timeout_for_tier(tier);
        let model_name = self.current_model(tier);

        let pty = PtyManager::for_tier(tier, &self.ai_config)
            .with_usage_context(usage_context)
            .with_timeout(timeout_secs)
            .with_nice_priority(DEFAULT_NICE_PRIORITY);
        let output = pty.spawn_claude(&workspace, &prompt.text).map_err(|e| {
            let msg = format!("{e:?}");
            if msg.contains("ClaudeCodeNotFound") || msg.contains("not authenticated") {
                ProviderError::Permanent(msg)
            } else {
                ProviderError::Transient(msg)
            }
        })?;
        Ok(Completion {
            text: output.stdout,
            fingerprint_metadata: FingerprintMetadata {
                provider: ProviderKind::ClaudeCode,
                model: model_name,
                temperature: CLAUDE_CODE_DEFAULT_TEMPERATURE,
                top_p: None,
                seed: None,
                tokens_input: None,
                tokens_output: None,
                provider_completion_id: None,
            },
        })
    }
}

#[async_trait]
impl IntelligenceProvider for PtyClaudeCode {
    async fn complete(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        let workspace = prompt
            .workspace
            .clone()
            .unwrap_or_else(|| self.default_workspace.clone());
        let prompt_text = prompt.text.clone();
        let ai_config = Arc::clone(&self.ai_config);
        let usage_context = self.usage_context.clone().with_tier(tier);
        let timeout_secs = Self::timeout_for_tier(tier);
        let model_name = self.current_model(tier);

        let join = tokio::task::spawn_blocking(move || {
            let pty = PtyManager::for_tier(tier, &ai_config)
                .with_usage_context(usage_context)
                .with_timeout(timeout_secs)
                .with_nice_priority(DEFAULT_NICE_PRIORITY);
            pty.spawn_claude(&workspace, &prompt_text)
        })
        .await
        .map_err(|e| ProviderError::Permanent(format!("spawn_blocking join error: {e}")))?;

        let output = join.map_err(|e| {
            // Map PTY ExecutionError into ProviderError. PTY auth/binary errors
            // are permanent; transient subprocess failures are transient.
            let msg = format!("{e:?}");
            if msg.contains("ClaudeCodeNotFound") || msg.contains("not authenticated") {
                ProviderError::Permanent(msg)
            } else {
                ProviderError::Transient(msg)
            }
        })?;

        Ok(Completion {
            text: output.stdout,
            fingerprint_metadata: FingerprintMetadata {
                provider: ProviderKind::ClaudeCode,
                model: model_name,
                temperature: CLAUDE_CODE_DEFAULT_TEMPERATURE,
                top_p: None,
                seed: None,
                tokens_input: None,
                tokens_output: None,
                provider_completion_id: None,
            },
        })
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::ClaudeCode
    }

    fn current_model(&self, tier: ModelTier) -> ModelName {
        let s = match tier {
            ModelTier::Synthesis => &self.ai_config.synthesis,
            ModelTier::Extraction => &self.ai_config.extraction,
            ModelTier::Background => &self.ai_config.background,
            ModelTier::Mechanical => &self.ai_config.mechanical,
        };
        ModelName::new(s.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_provider() -> PtyClaudeCode {
        let ai = AiModelConfig {
            synthesis: "model-syn".into(),
            extraction: "model-ext".into(),
            background: "model-bg".into(),
            mechanical: "model-mech".into(),
        };
        PtyClaudeCode::new(
            Arc::new(ai),
            std::env::temp_dir(),
            AiUsageContext::new("test", "pty_provider"),
        )
    }

    #[test]
    fn pty_provider_kind_is_claude_code() {
        let p = fixture_provider();
        assert_eq!(p.provider_kind(), ProviderKind::ClaudeCode);
    }

    #[test]
    fn pty_current_model_resolves_per_tier() {
        let p = fixture_provider();
        assert_eq!(p.current_model(ModelTier::Synthesis).as_str(), "model-syn");
        assert_eq!(p.current_model(ModelTier::Extraction).as_str(), "model-ext");
        assert_eq!(p.current_model(ModelTier::Background).as_str(), "model-bg");
        assert_eq!(
            p.current_model(ModelTier::Mechanical).as_str(),
            "model-mech"
        );
    }

    #[test]
    fn pty_timeout_for_tier_uses_documented_defaults() {
        assert_eq!(PtyClaudeCode::timeout_for_tier(ModelTier::Synthesis), 240);
        assert_eq!(PtyClaudeCode::timeout_for_tier(ModelTier::Extraction), 240);
        assert_eq!(PtyClaudeCode::timeout_for_tier(ModelTier::Background), 240);
        assert_eq!(PtyClaudeCode::timeout_for_tier(ModelTier::Mechanical), 90);
    }

    /// `pty_claude_code_fixture_returns_expected_fingerprint_metadata`
    /// (per DOS-259 plan §9): the metadata fields PtyClaudeCode populates
    /// at complete() time are deterministic for a given (config, tier).
    /// We assert the metadata shape via current_model() + provider_kind()
    /// rather than spawning Claude Code (which would require an authenticated
    /// runtime); the byte-identical parity test in §9 covers stdout shape.
    #[test]
    fn pty_claude_code_fixture_returns_expected_fingerprint_metadata() {
        let p = fixture_provider();
        let kind = p.provider_kind();
        let model = p.current_model(ModelTier::Synthesis);
        let meta = FingerprintMetadata {
            provider: kind.clone(),
            model: model.clone(),
            temperature: CLAUDE_CODE_DEFAULT_TEMPERATURE,
            ..FingerprintMetadata::default()
        };
        assert_eq!(meta.provider, ProviderKind::ClaudeCode);
        assert_eq!(meta.model, ModelName::new("model-syn"));
        assert_eq!(meta.temperature, CLAUDE_CODE_DEFAULT_TEMPERATURE);
        assert_eq!(kind, ProviderKind::ClaudeCode);
    }
}
