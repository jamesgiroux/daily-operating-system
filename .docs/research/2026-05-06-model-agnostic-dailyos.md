# Model-Agnostic DailyOS Research

**Date:** 2026-05-06
**Status:** Research note — candidate input for a v1.4.9 Linear project
**Linear context:** DailyOS team exists as `DOS`. No `v1.4.9` project exists yet. Recent adjacent projects include `v1.4.7 — Self-Healing v2` and `v1.4.8 — Reports as Shareable Intelligence`.
**Related:** `2026-05-06-privacy-first-local-models.md` refreshes the local model research with a stricter on-device, no-egress definition of "local."

---

## Executive Summary

DailyOS is closer to model agnosticism than the product surface implies, but not close enough to ship it as a setting flip.

The good news: the Rust backend already has an `IntelligenceProvider` trait with provider taxonomy for `ClaudeCode`, `Ollama`, `OpenAI`, and `Other`, plus prompt fingerprinting, replay fixtures, and a `PtyClaudeCode` adapter. This is the right architectural seam.

The gap: large production paths still instantiate `PtyManager` and call `claude --print` directly. Settings only allow the Claude Code model aliases `opus`, `sonnet`, and `haiku`. Onboarding, status checks, workspace scaffolding, docs, and the plugin marketplace still frame Claude Code as the AI engine.

The practical v1.4.9 thesis:

> Make DailyOS model-provider agnostic at the completion boundary while preserving local-first data ownership. Claude Code remains the default provider, Anthropic API key becomes a lower-friction cloud provider, and Ollama becomes the first local provider for mechanical/background/extraction workloads.

This should be scoped as a provider-runtime migration, not a model shootout.

---

## Current State In The Codebase

### 1. Claude Code is the default runtime, not just a connector

The README says AI features are powered by Claude Code and that the backend spawns Claude Code as a PTY subprocess. That matches the implementation.

`src-tauri/src/pty.rs` owns the Claude CLI runtime:

- Resolves a `claude` binary from PATH and common macOS locations.
- Checks Claude Code auth through the macOS Keychain entry `Claude Code-credentials`.
- Spawns `claude --model <model> --print <prompt>` in a PTY because Claude Code expects an interactive terminal.
- Explicitly forwards non-empty `ANTHROPIC_API_KEY` / `ANTHROPIC_AUTH_TOKEN`, and strips empty values so the child can fall back to Keychain.
- Records per-call token estimates, durations, operation labels, and model counts.

This makes Claude Code both the transport and part of the product contract.

### 2. The provider seam exists

`src-tauri/src/intelligence/provider.rs` defines:

- `ProviderKind::{ClaudeCode, Ollama, OpenAI, Other}`
- `ModelTier::{Synthesis, Extraction, Background, Mechanical}`
- `PromptInput`
- `Completion`
- `FingerprintMetadata`
- `ProviderError`
- `IntelligenceProvider`
- `ReplayProvider`

`src-tauri/src/intelligence/pty_provider.rs` adapts Claude Code into that trait through `PtyClaudeCode`.

This is a strong starting point because prompt identity already includes provider/model/sampling metadata. That matters for eval replay and trust/provenance: changing providers should intentionally produce a new fingerprint instead of silently overwriting an old Claude-backed result.

### 3. The seam is not universal yet

There are still direct `.spawn_claude(...)` calls in:

- `src-tauri/src/reports/generator.rs`
- `src-tauri/src/reports/swot.rs`
- `src-tauri/src/reports/book_of_business.rs`
- `src-tauri/src/risk_briefing.rs`
- `src-tauri/src/processor/email_actions.rs`
- `src-tauri/src/processor/transcript.rs`
- `src-tauri/src/processor/enrich.rs`
- `src-tauri/src/intel_queue.rs`
- `src-tauri/src/workflow/deliver.rs`
- `src-tauri/src/prepare/email_enrich.rs`

Some `intel_queue` paths already use `PtyClaudeCode::complete_blocking`, but background enrichment and repair retry still call `PtyManager` directly.

The migration target should be:

```rust
Arc<dyn IntelligenceProvider>::complete(prompt, tier)
```

not:

```rust
PtyManager::for_tier(...).spawn_claude(...)
```

### 4. Settings are Claude-alias specific

`AiModelConfig` only stores four opaque strings:

- `synthesis`
- `extraction`
- `background`
- `mechanical`

The defaults are:

- synthesis: `sonnet`
- extraction: `sonnet`
- background: `haiku`
- mechanical: `haiku`

But `services/settings.rs` validates models against only:

```rust
["opus", "sonnet", "haiku"]
```

The frontend mirrors this with:

```ts
const modelOptions = ["haiku", "sonnet", "opus"] as const;
```

This is not model-agnostic configuration. It is Claude Code model routing.

### 5. Onboarding and status are Claude-specific

Current onboarding calls the Claude step "The AI engine behind your briefings." It is skippable, but the UI and Settings still check `check_claude_status`, install `@anthropic-ai/claude-code`, and prompt `claude login`.

Backend status checks are also Claude-specific:

- `check_claude_status`
- `launch_claude_login`
- `install_claude_cli`
- `clear_claude_status_cache`

A model-agnostic product needs this surface to become "AI Provider" or "Intelligence Engine", with Claude Code as one provider option.

### 6. Workspace and plugin affordances are Claude-oriented

`write_managed_workspace_files()` creates `CLAUDE.md` and `.claude/settings.json` so Claude Code / Cowork understands the workspace. The plugin marketplace is explicitly a Claude Code plugin marketplace, with `dailyos` and `dailyos-writer` plugins installed via `claude plugin install`.

This does not block internal provider agnosticism, but it means there are two different scopes:

1. **DailyOS internal AI runtime:** can become provider-agnostic in v1.4.9.
2. **External assistant/plugin ecosystem:** remains Claude Code-specific until there is a second plugin target.

---

## OAuth vs API Key

There are two separate questions that should not be mixed:

### Claude Code with API key

Claude Code already supports `ANTHROPIC_API_KEY`. Official Claude Code docs say the key is sent as `X-Api-Key` and takes precedence over subscription OAuth in non-interactive `-p` mode when present. The local DailyOS PTY code already forwards non-empty `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN`.

So the simplest "API key mode" is not new provider work:

1. Store Anthropic API key in Keychain.
2. Inject it into the Claude Code child process env.
3. Keep calling `claude --print`.

This avoids the browser login friction and removes the Claude Pro/Max subscription requirement, but it does not remove the Claude Code dependency or make DailyOS provider-agnostic.

### Direct Anthropic API key provider

The cleaner cloud step is an `AnthropicMessagesProvider` that calls `/v1/messages` directly:

- No Node.js requirement.
- No Claude Code install requirement.
- No PTY.
- Cleaner timeout, error, streaming, and usage accounting.
- Structured tool / JSON behavior can be controlled directly.
- Easier enterprise posture: API keys, workload identity, or eventually gateway tokens.

This should sit behind `IntelligenceProvider`, not replace it.

### Privacy note

API key is not privacy-preserving by itself. It changes auth and billing, not locality. Claude Code docs state local Claude Code sends prompts and outputs over the network to interact with the LLM. Anthropic commercial/API traffic has different data policies than consumer Claude, and zero data retention can be available for eligible commercial arrangements, but that is contractual and provider-side. It is not the same as keeping data on the machine.

---

## Local Model Feasibility

### Privacy-first local runtime definition

There is an important distinction between "local-compatible provider" and "privacy-first local runtime."

For DailyOS, privacy-first local should mean:

- Model weights are downloaded to the user's machine.
- Inference runs on the user's machine.
- Prompts, retrieved context, transcripts, email content, outputs, embeddings, and tool traces do not leave the machine.
- The model server binds to `localhost` or an equivalent user-controlled local interface by default.
- No cloud fallback happens silently when the local model is unavailable.
- Telemetry is off by default for model runtime paths, or clearly separated from prompt/output data.
- Any model download/update traffic is explicit and separate from inference traffic.
- DailyOS can show a verifiable provider receipt: provider, model, local/cloud, endpoint, and whether network egress occurred.

That is a stronger product promise than "supports Ollama" or "supports OpenAI-compatible APIs." Ollama can be used in a privacy-first way, but only if DailyOS treats it as an on-device runtime and checks the endpoint, model availability, and data-flow behavior.

This framing also changes the value proposition. The point is not only model choice. It is:

- Lower recurring token spend for background work.
- Better privacy because private work data stays on device.
- Potentially lower latency for small/background calls.
- Better offline or degraded-network behavior.
- More useful idle-time processing because DailyOS can use local CPU/GPU without sending every minor task to a remote API.

Open source or open weight is helpful, but it is not the same as privacy. A closed local model with no egress can be more private than an open model hosted behind a remote API. A Chinese-origin open-weight model running fully local may protect prompt privacy better than a US-hosted API, but it may still raise supply-chain, license, provenance, or enterprise governance concerns. The privacy-first default should optimize for no inference egress first, then choose model families with acceptable provenance.

### Why Ollama is the best first local target

Ollama is the lowest-friction local provider target because:

- It runs on macOS, Windows, and Linux.
- It exposes local HTTP APIs.
- It has OpenAI-compatible `/v1/chat/completions`, `/v1/responses`, `/v1/models`, and `/v1/embeddings` support.
- It supports tool/function calling.
- Its OpenAI-compatible client path uses `base_url = http://localhost:11434/v1/` with an ignored API key, which maps well to a generic OpenAI-compatible provider.

The first local provider should probably be:

```text
ProviderKind::OpenAICompatible {
  base_url,
  api_key_ref,
  models_by_tier,
  capabilities
}
```

Then `OllamaProvider` can be mostly config:

```text
base_url = http://localhost:11434/v1
api_key = "ollama" // required by clients, ignored by Ollama
```

This also covers LM Studio, vLLM, SGLang, llama.cpp servers, OpenRouter-like gateways, and future enterprise LLM gateways if they expose OpenAI-compatible APIs.

### Local model candidates

These are candidate families, not recommendations to ship blindly.

| Candidate | Best first tier | Why it is interesting | Concern |
|---|---:|---|---|
| Qwen3-30B-A3B-Instruct-2507 | background / extraction | 30.5B total, 3.3B active MoE, 256K context, strong instruction following and tool usage | Chinese-origin model; local-only is better for privacy, but supply-chain/security review still needed |
| Qwen3-Coder-30B-A3B-Instruct | mechanical / code-like structured extraction | Designed for agentic coding and tool calls, 256K native context | Same governance concern; coding specialization may not match customer-success narrative tasks |
| Llama 3.3 70B Instruct | extraction / synthesis on high-end hardware | 128K context, strong open model baseline, Meta model card includes BFCL/tool-use benchmark | Too large for many consumer Macs; license is custom, not Apache/MIT |
| DeepSeek-R1 distills | reasoning experiments / repair retry | Strong reasoning family, distilled sizes available through Ollama | Reasoning verbosity and tool-call reliability can be awkward; Chinese-origin model risk is higher |
| Mistral Small class models | extraction / background | Open-weight European option, long context, good commercial posture | Need local runtime/model-card validation for the exact checkpoint before committing |

For the user's privacy requirement, the strongest position is:

1. Prefer non-Chinese open-weight models for the default local path.
2. Allow Chinese-origin local models only behind an explicit "advanced / bring your own model" setting.
3. Never call Chinese-hosted APIs for private DailyOS data.
4. Treat local model files as a supply-chain surface: pin model IDs, checksums where practical, and document provenance.
5. Define the privacy promise by inference egress, not by whether the weights are open source.

### Can a local model replace Haiku?

For some Haiku-class work, yes. For all Haiku-class work, not immediately.

Good first local candidates:

- Inbox classification.
- Email noise/relevance classification.
- Short file summaries.
- Keyword extraction.
- Draft action extraction from a constrained prompt.
- Background hygiene suggestions where deterministic validators gate writes.
- Enrichment repair retries that are checked by existing consistency/trust validation.

Risky first candidates:

- Executive briefing narrative.
- Full account/project synthesis.
- Stakeholder/political intelligence.
- Multi-document contradiction resolution.
- Anything that writes durable intelligence without deterministic validation.

DailyOS already has a good safety architecture for local models: provenance, trust bands, replay fixtures, source-aware rendering, and user corrections. The right rollout is tiered:

1. Local model can propose.
2. Deterministic validators and trust scoring decide whether output can be committed.
3. User-facing summaries must expose lower confidence when provider quality is unproven.

---

## Proposed v1.4.9 Project Shape

Candidate title:

> v1.4.9 — Model-Agnostic Intelligence Runtime

Mission:

> DailyOS should not require Claude Code as the only AI runtime. Users can choose Claude Code, Anthropic API key, OpenAI-compatible cloud providers, or a local Ollama-compatible model per tier, while DailyOS keeps local state, provenance, trust, and correction loops canonical.

### Workstream 1: Provider Configuration

Replace `AiModelConfig` with a provider-aware config:

```rust
struct AiProviderConfig {
    active_provider: ProviderId,
    providers: Vec<ProviderProfile>,
    tier_routing: TierRouting,
}

struct ProviderProfile {
    id: String,
    kind: ProviderKind,
    display_name: String,
    base_url: Option<String>,
    credential_ref: Option<KeychainRef>,
    capabilities: ProviderCapabilities,
}

struct TierRouting {
    synthesis: ModelRoute,
    extraction: ModelRoute,
    background: ModelRoute,
    mechanical: ModelRoute,
}

struct ModelRoute {
    provider_id: String,
    model: String,
}
```

Migration:

- Existing `sonnet` / `haiku` aliases become a default `claude_code` provider profile.
- Current four tier strings become `ModelRoute { provider_id: "claude_code", model }`.

### Workstream 2: Provider Implementations

Implement:

- `ClaudeCodeProvider` using existing `PtyClaudeCode`.
- `AnthropicMessagesProvider`.
- `OpenAIResponsesProvider` or `OpenAIChatCompletionsProvider`.
- `OpenAICompatibleProvider` for Ollama/local gateways.

Prefer one OpenAI-compatible transport with a provider capability matrix over bespoke Ollama-only code.

### Workstream 3: Call-Site Migration

Replace all direct `PtyManager::spawn_claude` production calls with provider invocation.

Priority order:

1. Mechanical/background paths.
2. Email enrichment and action extraction.
3. Transcript extraction.
4. Entity enrichment.
5. Reports and narrative synthesis.
6. External plugin/workflow surfaces.

The high-risk calls should move last because they are most sensitive to model behavior changes.

### Workstream 4: Provider Health and Onboarding

Replace Claude-specific status with a provider status matrix:

| Provider | Checks |
|---|---|
| Claude Code | binary found, auth found, model alias accepted |
| Anthropic API | key exists, `/v1/messages` smoke test, model access |
| OpenAI API | key exists, `/v1/responses` or `/v1/chat/completions` smoke test, model access |
| Ollama | local server reachable, model installed, context size, sample completion |

Settings should show provider health per route, not a single "Claude Code is ready" boolean.

### Workstream 5: Evaluation Harness

Every provider must pass tier-specific evals before it can be recommended:

- Golden prompt fixtures for each tier.
- JSON parse success rate.
- Schema validity.
- Source-ref preservation.
- Hallucination / cross-entity bleed checks.
- Latency and token/cost profile.
- Deterministic replay compatibility.

Use existing adversarial bundle patterns and provider replay infrastructure. This project should not ship provider selection without eval gates.

### Workstream 6: Privacy and Safety Policy

Provider UI needs explicit data-flow language:

- Local-only: prompts stay on device except model download/update checks.
- Claude Code subscription OAuth: prompts go to Anthropic under consumer/commercial account policy.
- Anthropic API key: prompts go to Anthropic API under API/commercial policy; ZDR may be available if the org has it.
- OpenAI API key: prompts go to OpenAI API; data controls and ZDR/MAM depend on organization approval.
- OpenAI-compatible custom URL: prompts go wherever that endpoint points.

No provider should be described as private unless DailyOS can prove the call stays on `localhost` or a user-controlled machine.

---

## Recommended First Slice

The smallest useful slice:

1. Add provider-aware config while preserving current behavior.
2. Add `OpenAICompatibleProvider` with Ollama as the first target.
3. Add a "Local Private Beta" route that only accepts localhost/user-controlled endpoints and never falls back to cloud silently.
4. Route only `mechanical` and `background` tiers through the provider trait.
5. Add Settings health checks for Ollama server + selected local model + endpoint locality.
6. Add eval fixtures for inbox classification, email action extraction, and background hygiene.
7. Keep `synthesis` and `extraction` defaulted to Claude Code until evals prove otherwise.

This would let DailyOS say:

> Claude Code is still the recommended full-quality engine, but simple/background tasks can run locally.

That is a credible, privacy-preserving v1 if the local route has a hard no-egress contract.

---

## Open Questions

1. Should v1.4.9 expose multiple providers in UI, or hide everything behind "Local beta" and "Claude Code" until evals pass?
2. Should Anthropic API direct-call ship before Ollama, since it removes Node/PTY friction without model-quality risk?
3. Do we want OpenAI as a first-party cloud provider, or only an OpenAI-compatible adapter where users bring their own base URL?
4. Do we require local model checksum/pinning before recommending Ollama?
5. Should generated reports record provider/model in their user-visible receipt?
6. Should external Claude Code plugins remain a separate "assistant plugins" feature instead of part of the internal AI provider story?
7. Should "local private" be a first-class product mode with hard egress checks, separate from generic provider routing?
8. Should DailyOS ship/manage a bundled model runtime, or detect and integrate with user-installed runtimes like Ollama first?

---

## Suggested Linear Breakdown

If this becomes `v1.4.9 — Model-Agnostic Intelligence Runtime`, suggested issues:

1. Research acceptance: finalize provider taxonomy, privacy policy, and tier rollout.
2. Config migration: provider profiles + tier routing, preserving current Claude aliases.
3. Keychain credentials: provider-scoped API key storage and deletion.
4. Provider implementation: Anthropic direct Messages API.
5. Provider implementation: OpenAI-compatible HTTP provider.
6. Provider implementation: Ollama health check and model discovery.
7. Call-site migration: mechanical/background tier.
8. Call-site migration: email action extraction and classification.
9. Eval harness: provider parity suite for mechanical/background.
10. Settings UI: provider status, model routing, and data-flow labels.
11. Docs: update README/setup from "Claude Code required" to provider matrix.
12. Decision gate: whether synthesis/report generation can leave Claude Code.

---

## Sources

### Local code references

- `src-tauri/src/pty.rs` — Claude Code PTY runtime, auth/env handling, usage ledger, model tiers.
- `src-tauri/src/intelligence/provider.rs` — provider trait, provider taxonomy, replay/fingerprint model.
- `src-tauri/src/intelligence/pty_provider.rs` — Claude Code provider adapter.
- `src-tauri/src/types.rs` — current `AiModelConfig` defaults.
- `src-tauri/src/services/settings.rs` — current model validation limited to `opus`, `sonnet`, `haiku`.
- `src/features/settings-ui/SystemStatus.tsx` — current model picker limited to Claude aliases.
- `src/components/onboarding/chapters/ClaudeCode.tsx` — Claude-specific onboarding.
- `src-tauri/src/commands/app_support.rs` — Claude status, login, installer.
- `src-tauri/src/util.rs` — managed `CLAUDE.md` / `.claude/settings.json`.
- `plugins/README.md` — Claude Code plugin marketplace framing.

### External references checked 2026-05-06

- Claude Code authentication: https://code.claude.com/docs/en/authentication
- Claude Code environment variables: https://code.claude.com/docs/en/env-vars
- Claude Code model configuration: https://code.claude.com/docs/en/model-config
- Claude Code data usage: https://code.claude.com/docs/en/data-usage
- Claude API authentication: https://platform.claude.com/docs/en/manage-claude/authentication
- Claude API data retention: https://platform.claude.com/docs/en/manage-claude/api-and-data-retention
- OpenAI Responses API: https://platform.openai.com/docs/api-reference/responses/object
- OpenAI data controls: https://developers.openai.com/api/docs/guides/your-data
- Ollama documentation: https://docs.ollama.com/
- Ollama OpenAI compatibility: https://docs.ollama.com/api/openai-compatibility
- Ollama tool calling: https://docs.ollama.com/capabilities/tool-calling
- Llama 3.3 70B Instruct model card: https://huggingface.co/meta-llama/Llama-3.3-70B-Instruct
- Qwen3-30B-A3B-Instruct-2507 model card: https://huggingface.co/Qwen/Qwen3-30B-A3B-Instruct-2507
- Qwen3-Coder-30B-A3B-Instruct model card: https://huggingface.co/Qwen/Qwen3-Coder-30B-A3B-Instruct
- DeepSeek-R1 model card: https://huggingface.co/deepseek-ai/DeepSeek-R1
