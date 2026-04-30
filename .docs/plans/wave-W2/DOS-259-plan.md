# Implementation Plan: DOS-259

## Revision history

- v2 (2026-04-28) — cycle 1 revision pass. Addressed all High findings + load-bearing Mediums from L0 triangle. Trait shape now frozen from ADR-0106 §3 + ADR-0091 in §3 (not §10). Parity test concrete. Replay routing site named. W2-A merge order resolved. Replay/W4-B scope line drawn.
- v3 (2026-04-29) — L6-authorized cycle-3 revision. Closed cycle-2 consult F2 (provider routing seam moved from &ServiceContext to &AbilityContext per ADR-0104 split + DOS-259 ticket §Architectural surfaces touched).
- v1 (2026-04-28) — initial L0 plan.

## 1. Contract restated

DOS-259 is a pure extraction refactor for the v1.4.0 spine: introduce the shared `IntelligenceProvider` trait, move the current Claude Code PTY and Glean intelligence-generation paths behind it, and preserve behavior. The Linear ticket pins the acceptance surface: `pub trait IntelligenceProvider` lives at `src-tauri/src/intelligence/provider.rs`; `PtyClaudeCode` and Glean implement it; text-only callers use `provider.complete(...).await?.text`; replay exists under `#[cfg(test)]` or Evaluate mode; existing enrichment and meeting-prep behavior stays byte-identical.

Scope limits stay strict. DOS-259 does not implement production prompt fingerprinting; DOS-213 consumes this hook later. It does not add OpenAI/Ollama providers. It does not ship the W4-B/DOS-216 evaluation fixture format. It does not change claims, provenance storage, or derived-state writes.

Code-reality note: current PTY orchestration is in `intel_queue.rs` helpers called by `services/intelligence.rs`, while `services/intelligence.rs:226` has an inline Glean construction for manual refresh. This plan preserves service-layer writes and extracts only provider invocation.

## 2. Approach

Create `src-tauri/src/intelligence/provider.rs` for the trait, `Completion`, `PromptInput`, `FingerprintMetadata`, `ProviderError`, `ProviderKind`, `ModelName`, `ReplayProvider`, and `select_provider(ability_ctx, tier)`. Create `src-tauri/src/intelligence/pty_provider.rs` and extract the PTY completion path into `PtyClaudeCode`. Refactor `src-tauri/src/intelligence/glean_provider.rs` so its intelligence-generation path implements `complete()` while its Glean-specific discovery and leading-signal helpers remain module-local helpers, not provider-selection sites.

Current grep snapshot against `src-tauri/src/` on 2026-04-28:

- `rg -n 'PtyManager::for_tier' src-tauri/src/` -> 23 matches.
- `rg -n 'GleanIntelligenceProvider::new' src-tauri/src/` -> 4 matches.
- `rg -n '\.complete\(.*tier' src-tauri/src/` -> 0 matches before this refactor.

Production migration is intentionally small. Migrate the 2 ADR-0091 intelligence-generation PTY sites, `intel_queue.rs:1607` (parallel extraction) and `intel_queue.rs:1819` (legacy synthesis), plus provider selection in `services/intelligence.rs`. Remove the inline `GleanIntelligenceProvider::new(endpoint)` at `services/intelligence.rs:226` and the main batch-enrichment Glean construction at `intel_queue.rs:1403` from caller code by routing through the AppState-owned provider `Arc` bridge until `AbilityContext` is available. Target post-refactor production `provider.complete(..., tier)` consumers: 2 (`services/intelligence.rs` and `intel_queue.rs`), with test-only calls excluded. Leave the remaining 21 PTY-direct matches in reports, processors, executors, devtools, background maintenance, repair retry, and other Claude-specific workflows alone; ADR-0091 explicitly scoped those deliberate exceptions outside the intel-queue provider abstraction. Glean account discovery at `commands/integrations.rs:3769` and leading-signal enrichment at `intel_queue.rs:894` remain Glean-specific product calls, not `IntelligenceProvider::complete()` consumers.

Replay scope is W2-B only at the trait/routing layer: `ReplayProvider` takes an in-memory `HashMap<Hash, Completion>` or constructor-supplied lookup closure. Fixture file format, on-disk layout, anonymization, capture governance, and CI harness integration are explicitly W4-B/DOS-216.

## 3. Key decisions

ADR-pinned trait shape is not an open question. ADR-0106 §3 says:

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

ADR-0091 supplies the Send/Sync bound:

```rust
#[async_trait]
pub trait IntelligenceProvider: Send + Sync {
    async fn complete(
        &self,
        prompt: &str,
        tier: ModelTier,
    ) -> Result<String, ProviderError>;
}
```

W2-B combines those decisions: the shipped trait is `IntelligenceProvider: Send + Sync`; `complete()` returns `Result<Completion, ProviderError>`; `Completion` has exactly `text: String` and `fingerprint_metadata: FingerprintMetadata`. Cost and latency fields are deferred, not open.

`ProviderKind` reuses ADR-0106's enum surface: `ClaudeCode | Ollama | OpenAI | Other(&'static str)`. Glean is `Other("glean")`; replay is `Other("replay")` and only available under `#[cfg(test)]` or Evaluate-mode wiring. `ModelTier` uses ADR-0091's provider tiers: `Synthesis | Extraction | Mechanical`. The ticket's `Fast` / `Standard` / `Max` names are illustrative. The current PTY-only `Background` tier remains outside the provider contract unless a later ADR expands the enum.

Single source of ability-context provider selection is `intelligence::provider::select_provider(ctx: &AbilityContext, tier: ModelTier) -> Arc<dyn IntelligenceProvider>`. ServiceContext.execution_mode is the only thing the factory reads from ServiceContext-adjacent context; everything else is AbilityContext-owned. Factory routing: Live returns the configured AppState provider `Arc`, Evaluate returns replay, and Simulate returns a non-live provider whose `complete()` fails closed with `ProviderError::ModeNotSupported` rather than invoking PTY or HTTP. Pick: AppState keeps the configured live provider `Arc` per ADR-0091 and swaps it on settings changes; `select_provider()` does not replace storage, it enforces per-call mode routing in ability-execution contexts.

Bridge for early callers: `intel_queue.rs` and `services/intelligence.rs` do not have an `AbilityContext` today. In ability-execution contexts (W3+ when `AbilityContext` exists), callers use `select_provider(ability_ctx, tier)`. Current/early callers without `AbilityContext` route via the AppState-owned provider `Arc` per ADR-0091; the Arc swap on settings change continues to work. When `AbilityContext` lands in W3-A (DOS-210 ability registry), those early callers migrate to the AbilityContext-routed factory.

## 4. Security

The trait is a trust boundary. `PtyClaudeCode` must not leak PTY handles or prompt bodies into logs. Glean endpoint secrets and session details stay encapsulated in `glean_provider.rs`. `ProviderError` messages must not include prompt text, completion text, customer content, or raw provider payloads; logs use provider kind, model, tier, and non-sensitive error class only.

Evaluate mode must be structurally incapable of network or PTY invocation. Missing replay data returns a fixture-missing `ProviderError`; it never falls through to Live. Simulate mode is non-generative for this refactor and must fail closed through `ModeNotSupported`. The security argument still holds: Evaluate-mode replay routing is structurally enforced via the AbilityContext-bearing factory plus the ADR-0091 AppState `Arc`-swap pattern.

## 5. Performance

The new cost is one trait dispatch and possibly one boxed async future per completion, which is negligible beside PTY subprocess or Glean HTTP latency. Provider selection happens once per provider invocation, not inside inner parse/merge loops. Replay is in-memory in W2-B and has no filesystem or network startup cost. `FingerprintMetadata` is populated from provider-known fields only; token counts remain optional for PTY per ADR-0106.

## 6. Coding standards

Providers may invoke external systems and return `Completion`; they must not write DB rows, `intelligence.json`, signals, claims, or UI events directly. Mutations stay in services and existing queue write paths, preserving the services-only rule. Intelligence Loop 5-question check: no schema, signal type, health-scoring rule, briefing surface, or feedback hook is added here.

No provider module may introduce direct `Utc::now()` or `thread_rng()`. W2-B extends the W2-A lint coverage to include `src-tauri/src/intelligence/{provider,pty_provider,glean_provider}.rs`, closing the gap left by the default `services/` + `abilities/` glob. Fixtures use generic data only.

## 7. Integration with parallel wave-mates

W2-B opens first and restructures `services/intelligence.rs` / `intel_queue.rs` around provider invocation before W2-A sweeps mutation gates. W2-A then rebases on the smaller mutation surface and adds `check_mutation_allowed()` to the remaining service mutations. This order matches the wave-plan coordination hint and avoids W2-B rebasing across W2-A's broad `services/intelligence.rs` edits.

W2-B does not edit `src-tauri/src/services/context.rs`; W2-A owns `ServiceContext`, `ExecutionMode`, `Clock`, and `SeededRng`. `ServiceContext` does not own the provider `Arc`; W3-A's `AbilityContext` owns the provider `Arc` and provider seam. W2-B ships the trait, implementations, and routing factory; the routing factory is wired into `AbilityContext` when the W3-A/DOS-210 registry lands. Until then, callers use the AppState-`Arc` bridge per ADR-0091. Shared-file owner note: `services/intelligence.rs` is the collision point; W2-B removes provider orchestration, W2-A gates the remaining writes.

## 8. Failure modes + rollback

Main risk is behavior drift in an extraction that claims no behavior change. Parity evidence is concrete: commit pinned PTY and Glean response fixtures plus pre-refactor baseline `IntelligenceJson` snapshots alongside the PR. The parity harness replays the same prompt and response through both the pre-refactor inline path and the post-refactor provider path (`PtyClaudeCode::complete()` and Glean `complete()`), then compares canonical serialized `IntelligenceJson` bytes. It also asserts completion text byte equality and prompt-template identity/hash equality where the W2 hook supplies it. DOS-213 still owns production canonical hash computation.

Drop Trust Compiler shadow parity from W2-B: W4-A does not exist yet. Meeting prep is either untouched and covered by a grep/non-touch assertion, or if the implementation touches it, it must get the same byte-identical fixture parity before PR. Rollback is mechanical: revert caller migration to inline PTY/Glean paths while leaving provider files unused. No SQL migration is required.

## 9. Test evidence to be produced

Required tests: `replay_provider_returns_canned_completion`, `evaluate_mode_never_invokes_live_provider`, `provider_selection_is_single_source_for_tier`, `pty_claude_code_fixture_returns_expected_fingerprint_metadata`, `glean_provider_fixture_returns_expected_fingerprint_metadata`, `pty_provider_parity_fixture_intelligence_json_byte_identical`, `glean_provider_parity_fixture_intelligence_json_byte_identical`, and `provider_complete_concurrent_invocations_all_succeed`.

The concurrency test drives N simultaneous `.complete()` calls against a fixture-backed provider and proves the `Send + Sync` invariant from ADR-0091. The W2 merge-gate artifact remains `cargo clippy -- -D warnings && cargo test` plus the broader wave gate `pnpm tsc --noEmit`. Suite E contribution is replay-provider coverage on the stub/in-memory fixture. No Suite S or Suite P contribution is expected from this PR beyond the standard gate.

## 10. Open questions

No L6-blocking question remains for cycle 1. Only implementation-placement ambiguity remains: `ModelName` can live in `provider.rs` or a shared intelligence model module, but it must stay the ADR-0106 newtype and must not change trait shape.

Committed DOS-304 read: Linear still shows DOS-259 blocked by DOS-304, but `v1.4.0-waves.md` classifies DOS-304 as a contract-only decision-reference ticket whose decisions are already absorbed into the 22-issue spine. W2-B does not gate on DOS-304 resolution unless DOS-304 receives a new amendment after 2026-04-28 that materially changes the DOS-259 contract.
