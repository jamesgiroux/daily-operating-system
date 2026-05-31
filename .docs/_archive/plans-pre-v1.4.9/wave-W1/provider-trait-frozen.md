# Provider Trait Frozen Contract

Status: frozen for W2-A / consumed by W2-J

ADR-0106 extends ADR-0091 by changing the provider contract from a raw string response to a completion envelope:

```rust
#[async_trait]
pub trait IntelligenceProvider: Send + Sync {
    async fn complete(
        &self,
        prompt: PromptInput,
        tier: ModelTier,
    ) -> Result<Completion, ProviderError>;

    fn provider_kind(&self) -> ProviderKind;
    fn current_model(&self, tier: ModelTier) -> ModelName;
}
```

`Completion` is the stable return type:

```rust
pub struct Completion {
    pub text: String,
    pub fingerprint_metadata: FingerprintMetadata,
}
```

The `.text` field remains public as the migration accessor for older call sites that expected `String`.

`FingerprintMetadata` is provider-owned invocation metadata:

```rust
pub struct FingerprintMetadata {
    pub provider: ProviderKind,
    pub model: ModelName,
    pub temperature: f32,
    pub top_p: Option<f32>,
    pub seed: Option<u64>,
    pub tokens_input: Option<u32>,
    pub tokens_output: Option<u32>,
    pub provider_completion_id: Option<String>,
}
```

`PromptInput` carries rendered prompt text plus optional template identity, template hash, canonical JSON inputs, and PTY workspace. Providers execute `PromptInput.text`; fingerprinting combines the template fields with provider metadata.

Fingerprinting is frozen behind `intelligence::prompt_fingerprint`:

- `replay_fixture_key(prompt, metadata)` computes the canonical replay key.
- `prompt_fingerprint_from_completion(completion, prompt, id, version)` constructs `Provenance.prompt_fingerprint`.
- Low-level `canonical_prompt_hash` is provider-boundary only and should not be called by abilities.

Replay behavior is fail-closed. `Simulate` and `Evaluate` route to replay when configured; missing fixture entries return `ProviderError::FixtureMissingCompletion { hash }` and never fall through to a live provider.

The canonical hash input is the canonical template hash, canonical JSON inputs, provider, model, temperature, top_p, and seed. Template canonicalization normalizes Unix line endings, trims trailing whitespace, and uses one trailing newline.
