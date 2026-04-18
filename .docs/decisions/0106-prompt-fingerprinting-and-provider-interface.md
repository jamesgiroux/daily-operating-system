# ADR-0106: Prompt Fingerprinting and Provider Interface Extension

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0091](0091-intelligence-provider-abstraction.md), [ADR-0105](0105-provenance-as-first-class-output.md)  
**Depends on:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md)  
**Related:** [ADR-0110](0110-evaluation-harness-for-abilities.md) (consumer of fingerprints)

## Context

[ADR-0105](0105-provenance-as-first-class-output.md) §1 declares that every ability that invokes the intelligence provider records a `PromptFingerprint` in its provenance envelope. [ADR-0110](0110-evaluation-harness-for-abilities.md) (forthcoming) uses the fingerprint to distinguish prompt regressions from input regressions during evaluation. [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §6 requires replay providers to produce deterministic completions in `Evaluate` mode.

[ADR-0091](0091-intelligence-provider-abstraction.md) defines `IntelligenceProvider::complete(prompt, tier) -> String`. This signature does not expose the metadata — provider name, model, token counts, seed, prompt template version — that `PromptFingerprint` requires. ADR-0091 was scoped to the intel queue; ADR-0102 expanded provider use to all Transform abilities. This ADR formally amends ADR-0091's trait to expose fingerprint-capable metadata and specifies the fingerprint shape and canonicalization rules.

## Decision

### 1. The `PromptFingerprint` Shape

```rust
pub struct PromptFingerprint {
    pub provider: ProviderKind,            // Enum: ClaudeCode | Ollama | OpenAI | Other(&'static str)
    pub model: ModelName,                  // Newtype over String; runtime-configurable
    pub prompt_template_id: PromptTemplateId, // Stable ID of the prompt template
    pub prompt_template_version: PromptVersion, // Semver per template
    pub canonical_prompt_hash: Hash,       // SHA256 of canonicalized template + inputs
    pub temperature: f32,
    pub top_p: Option<f32>,
    pub seed: Option<u64>,                 // Required in Evaluate mode
    pub tokens_input: Option<u32>,
    pub tokens_output: Option<u32>,
    pub provider_completion_id: Option<String>, // When the provider supplies a stable ID
}
```

**ModelName is a newtype, not `&'static str`.** Models are configurable at runtime per [ADR-0091](0091-intelligence-provider-abstraction.md); a static string would prevent user-selected models from being captured.

**`provider_completion_id` is optional.** Claude Code over PTY and local Ollama may not expose a stable completion ID; OpenAI and hosted Anthropic APIs do. Consumers MUST NOT depend on it being present.

### 2. Canonicalization Rules for `canonical_prompt_hash`

The hash must be stable across whitespace, formatting, and JSON key ordering changes that do not affect prompt semantics:

1. Prompt templates are stored in a normalized form: Unix line endings, no trailing whitespace, single trailing newline, no tabs (converted to four spaces).
2. Input substitution into templates uses canonical JSON for object-valued inputs: keys sorted alphabetically, whitespace minimized, no comments.
3. The hash input is the concatenation of: prompt template ID, prompt template version, canonicalized template bytes, canonical JSON of input values, provider kind, model name, temperature bytes (fixed-width `f32` big-endian), top_p (if set), seed (if set).
4. Provider response data is NOT included in the hash. Two identical prompts with different completions produce the same hash.

### 3. Amendment to `IntelligenceProvider`

ADR-0091's trait is amended. The `complete()` method returns a `Completion` struct instead of `String`:

```rust
pub struct Completion {
    pub text: String,
    pub fingerprint_metadata: FingerprintMetadata,
}

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

#[async_trait]
pub trait IntelligenceProvider {
    async fn complete(&self, prompt: PromptInput, tier: ModelTier) -> Result<Completion, Error>;
    fn provider_kind(&self) -> ProviderKind;
    fn current_model(&self, tier: ModelTier) -> ModelName;
}
```

`PromptInput` carries the template ID, version, and pre-materialized input values so the provider can compute `tokens_input` accurately. Provider implementations produce `FingerprintMetadata` from their actual invocation; abilities combine it with the prompt-template hash to produce the full `PromptFingerprint` stored in provenance.

**Backward compatibility with ADR-0091.** Existing callers that receive only the text can do so via a thin helper `provider.complete(prompt, tier).await?.text`. The text field is unchanged. The added metadata is additive.

### 4. Replay Semantics

[ADR-0104](0104-execution-mode-and-mode-aware-services.md) §6 requires replay providers in `Evaluate` mode. This ADR specifies:

**Replay is keyed by `canonical_prompt_hash`, not by seed.** A replay provider's fixture maps `canonical_prompt_hash → Completion`. Looking up a completion by hash gives the fixture's recorded response. If the hash is missing from the fixture, the replay provider returns `Error::FixtureMissingCompletion { hash }`.

**Seed is for live generation determinism, not replay.** When `Evaluate` mode drives a live provider (e.g., running a regression test against a real model to capture a new fixture), `seed` is mandatory; the provider passes it through to the model API. When `Evaluate` mode drives a replay provider, `seed` is retained in the fingerprint for audit but does not drive the replay.

**Fixture construction.** Running an ability in `Live` mode with a tracing wrapper records every provider invocation (prompt template ID, inputs, metadata, completion). The captured sequence is the replay fixture for future `Evaluate` runs. [ADR-0110](0110-evaluation-harness-for-abilities.md) specifies fixture storage, anonymization, and refresh cadence.

### 5. Prompt Template Registry

Prompt templates have stable IDs and versions, stored at `src-tauri/src/abilities/prompts/`:

```
prompts/
├── prepare_meeting_prep.v2.txt         # "prepare_meeting_prep" @ v2.0.0
├── prepare_meeting_prep.v3.0.txt       # "prepare_meeting_prep" @ v3.0.0
├── detect_risk_shift.v1.txt
└── manifest.toml                        # Maps template IDs to paths and versions
```

Prompt changes are independent of ability schema versions ([ADR-0102](0102-abilities-as-runtime-contract.md) §8). A prompt bump produces a new fingerprint without bumping the ability version. `prompt_template_version` in the fingerprint lets consumers distinguish prompt-driven output changes from schema-driven ones.

### 6. Prompt Fingerprint Usage in Evaluation

[ADR-0110](0110-evaluation-harness-for-abilities.md) uses fingerprints for regression classification:

- Same inputs, same `prompt_template_version`, different outputs → provider drift or fixture inconsistency
- Same inputs, different `prompt_template_version`, different outputs → expected prompt change (reviewer approves or rejects)
- Different inputs, same fingerprint otherwise → real input change
- Different `canonical_prompt_hash`, same template version → canonicalization bug (template edit that didn't bump version, or input-materialization drift)

## Consequences

### Positive

1. **Provider metadata captured consistently.** Every LLM invocation records provider, model, temperature, seed, tokens, and completion ID where available.
2. **Evaluation distinguishes prompt regressions from input regressions.** The fingerprint classification in §6 is structurally sound.
3. **Replay semantics clear.** Fixture lookup by hash, with seed reserved for live generation determinism.
4. **Prompt template versioning separate from ability versioning.** Prompt iteration doesn't churn ability schemas.
5. **Canonicalization prevents spurious fingerprint churn.** Whitespace and JSON-key reordering no longer change the hash.

### Negative

1. **`IntelligenceProvider` trait amendment is a breaking change for existing consumers.** ADR-0091's callers must migrate to the `Completion` return type. Helper `.text` field minimizes churn.
2. **Every provider implementation must supply `FingerprintMetadata`.** Claude Code PTY and Ollama implementations need to extract model name, temperature, tokens from their response flows. Non-trivial for PTY-based providers that historically returned only text.
3. **Prompt template registry adds infrastructure.** Template file naming, manifest, versioning conventions are new concepts.
4. **Canonicalization rules add implementation complexity.** Normalization of templates and canonical JSON of inputs must be identical across hash producer and consumer.

### Risks

1. **Provider metadata incompleteness.** A PTY provider cannot extract token counts reliably. Mitigation: metadata fields are `Option<u32>`. Consumers handle missing values.
2. **Fingerprint churn from canonicalization bugs.** A whitespace-normalization difference between producer and consumer causes spurious new hashes. Mitigation: shared canonicalization library, unit tests on normalization.
3. **Prompt template drift from the manifest.** A contributor edits a template file without bumping its version. Mitigation: pre-commit hook hashes templates and refuses versions with changed hashes.
4. **Fixture bloat.** Every prompt+input combination recorded creates a replay entry. Mitigation: fixtures are scoped to specific evaluation tasks, not all traffic. [ADR-0110](0110-evaluation-harness-for-abilities.md) details retention and pruning.
5. **Replay fixture staleness.** Live provider behavior drifts from fixture. Mitigation: fixtures are refreshed on a declared cadence. Staleness is flagged at evaluation time when a replay produces unexpected outputs vs. a live spot-check.

## References

- [ADR-0091: IntelligenceProvider Abstraction](0091-intelligence-provider-abstraction.md) — Amended by this ADR: `complete()` returns `Completion` instead of `String`.
- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Prompt changes do not bump ability versions; version discipline in §8.
- [ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md) — §6 requires determinism in `Evaluate` mode, satisfied by replay keyed by `canonical_prompt_hash`.
- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — §1 `PromptFingerprint` shape referenced; this ADR provides the definition.
- **ADR-0110 (forthcoming): Evaluation Harness for Abilities** — Consumes fingerprints for regression classification.
