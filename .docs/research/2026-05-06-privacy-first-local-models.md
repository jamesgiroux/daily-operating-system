# Privacy-First Local Model Research For DailyOS

**Date:** 2026-05-06
**Status:** Research addendum to `2026-05-06-model-agnostic-dailyos.md`
**Purpose:** Identify local model candidates for a model-agnostic DailyOS runtime where "local" means on-device inference with no prompt egress.

---

## Framing

The definition of done for v1.4.9 should still include model agnosticism:

- Claude / Anthropic.
- ChatGPT, Codex, and OpenAI APIs.
- OpenAI-compatible providers.
- Ollama / LM Studio / llama.cpp / MLX.
- Local or self-hosted user-controlled models.

But "local" needs a stricter definition than "a provider reachable from DailyOS." If DailyOS shells out to Claude Code, Codex, ChatGPT, or another CLI that forwards prompts to a remote inference service, that is a local process but not local inference. It should be labeled as cloud or remote inference because private DailyOS context leaves the device for model processing.

For this research, "local private" means:

- Model weights are downloaded to the user's machine.
- Inference runs on the user's machine.
- Prompt data, retrieved context, transcripts, email content, tool traces, outputs, and embeddings do not leave the machine during inference.
- The inference endpoint is loopback-only by default, such as `localhost`, `127.0.0.1`, `::1`, or a Unix socket.
- There is no silent cloud fallback.
- Model download/update traffic is explicit and separate from inference.
- DailyOS records a provider receipt with provider, model, endpoint class, execution location, and whether network egress was expected.

This creates two parallel product goals:

1. **Provider agnosticism:** the architecture can swap providers and route by tier.
2. **Privacy-first local mode:** a first-class route that enforces on-device inference.

---

## Evaluation Criteria

Local model candidates should be evaluated on:

- **Privacy fit:** Can run fully offline after download; no hosted API required.
- **Runtime fit:** Works through Ollama, LM Studio, llama.cpp, MLX, vLLM, SGLang, or ONNX without bespoke DailyOS runtime code.
- **Structured output:** Can reliably produce JSON/schema-bound outputs for extraction and actions.
- **Tool/function calling:** Useful for future local agent workflows.
- **Context length:** Enough for email/transcript/report context, or compatible with chunked retrieval.
- **Latency and memory:** Realistic on common Apple Silicon and workstation machines.
- **License/commercial posture:** Viable for a commercial desktop product.
- **Provenance/governance:** Model origin, supply-chain risk, gated downloads, and enterprise acceptability.

Open source or open weight is useful, but privacy comes from the inference boundary. A remote open-weight model still leaks prompts. A downloaded local model with no egress is private for prompt processing even if its license is not fully open-source.

---

## Shortlist

### 1. OpenAI `gpt-oss-20b`

**Recommendation:** First model to test for DailyOS Local Private Beta.

Why it fits:

- OpenAI open-weight model, Apache 2.0.
- Designed for local or specialized use cases.
- 21B parameters with 3.6B active parameters.
- Model card says the MXFP4 setup lets the 20B model run within 16GB of memory.
- Supports configurable reasoning effort, function calling, structured outputs, and agentic workflows.
- Runs through Ollama and LM Studio, so DailyOS can use an OpenAI-compatible local endpoint instead of custom model code.
- Strong provenance for users who trust OpenAI but want inference to stay on device.

Best DailyOS tiers:

- `mechanical`
- `background`
- `extraction` after evals
- possible lightweight `synthesis` only after narrative-quality evals

Concerns:

- It uses OpenAI's Harmony response format; DailyOS should rely on runtimes that apply the correct template or explicitly validate the prompt format.
- Full chain-of-thought availability is a developer/debugging issue, not something DailyOS should expose to end users.
- It is not ChatGPT and is not served through the OpenAI API. Treat it as a local OpenAI-origin model, separate from cloud OpenAI/Codex routes.

DailyOS take:

`gpt-oss-20b` is the most compelling default local candidate because it matches the provider-agnostic direction and the privacy-first goal at the same time. It gives DailyOS an OpenAI-origin local route while keeping ChatGPT/Codex/OpenAI API as separate cloud routes.

### 2. Mistral Small 3.2 24B Instruct

**Recommendation:** Primary non-US local candidate to test.

Why it fits:

- Apache 2.0.
- 24B class model with improved instruction following, fewer repetition failures, and stronger function-calling template behavior compared with the previous Mistral Small release.
- Mistral's examples use OpenAI-compatible local serving through vLLM.
- Strong European provenance and a cleaner enterprise governance story than Chinese-origin models.
- Good candidate for structured extraction and local report drafting experiments.

Best DailyOS tiers:

- `background`
- `extraction`
- `mechanical`
- possible `synthesis` on stronger machines

Concerns:

- Heavier than 7B-14B models.
- Needs real Apple Silicon / consumer GPU testing before recommending as a default.
- Tool calling should be validated against DailyOS schemas, not assumed from benchmark claims.

DailyOS take:

This is the strongest privacy-first alternative to `gpt-oss-20b` if we want a non-OpenAI local model with commercial-friendly licensing and good function-calling posture.

### 3. IBM Granite 4.0 H Tiny / Small

**Recommendation:** Enterprise-safe low-resource lane.

Why it fits:

- Apache 2.0.
- IBM provenance and enterprise-oriented model cards.
- Granite 4.0 H Tiny is a 7B long-context instruct model with documented capabilities for summarization, classification, extraction, RAG, code-related tasks, and function calling.
- The Granite 4.0 H Small base model is positioned for long-context text generation, summarization, classification, extraction, QA, and code completion.
- This family is appealing for business users who care about provenance and licensing more than leaderboard scores.

Best DailyOS tiers:

- `mechanical`
- `background`
- low-risk extraction
- local classification and hygiene jobs

Concerns:

- Needs quality testing against DailyOS-specific prompts.
- The Tiny model may be too weak for nuanced stakeholder intelligence.
- The Small family may require more RAM and more runtime validation.

DailyOS take:

Granite is a strong candidate for "boring but governable" local intelligence. It may not beat the best models, but it has the right enterprise posture for privacy-first business workflows.

### 4. Microsoft Phi-4 Mini / Phi-4 Reasoning Plus

**Recommendation:** Low-resource and reasoning-specialist lane.

Why it fits:

- MIT license.
- Phi-4 Mini has optimized ONNX builds and a 128K context claim in the ONNX model card.
- Microsoft publishes ONNX variants for CPU/GPU across Windows, Linux, Mac desktops, and mobile CPUs.
- Phi-4 Reasoning Plus is a 14B model with strong reasoning benchmarks and MIT license.

Best DailyOS tiers:

- `mechanical`
- `background`
- repair/retry prompts
- constrained reasoning jobs

Concerns:

- Reasoning models can generate too many tokens and add latency.
- Phi-4 Reasoning Plus has a 32K context length, which may be enough for focused jobs but not full transcript/report synthesis.
- Need strict output validators because small models can be brittle on JSON and source-ref requirements.

DailyOS take:

Phi is not my first default local model, but it is a useful lane for low-resource installs and specific local reasoning/repair tasks.

### 5. Google Gemma 3 12B / 27B IT

**Recommendation:** Secondary candidate, especially for multimodal or Google-trusted users.

Why it fits:

- Google provenance.
- Gemma 3 includes 1B, 4B, 12B, and 27B variants.
- Hugging Face's Gemma 3 writeup describes up to 128K context for 4B, 12B, and 27B variants, plus multimodal input for those sizes.
- The Hugging Face model pages include local serving examples through vLLM and OpenAI-compatible APIs.

Best DailyOS tiers:

- `background`
- summarization
- document/image-adjacent workflows if DailyOS later uses local vision
- extraction after schema evals

Concerns:

- Gemma license is not Apache/MIT.
- Hugging Face access is gated behind Google license acceptance.
- Need practical testing for structured extraction and tool calling.

DailyOS take:

Gemma is worth testing, but I would not make it the default privacy-first model until license/product-distribution implications are clear.

---

## High-End Or Conditional Candidates

### OpenAI `gpt-oss-120b`

Strong local workstation candidate, not a default desktop model.

- Apache 2.0.
- 117B parameters with 5.1B active parameters.
- Model card positions it for a single 80GB GPU class system.
- Good candidate for "full local synthesis" on very high-end hardware.

This is not realistic for most DailyOS users, but it matters for enterprise desktops, private workstations, and future hardware.

### Meta Llama 3.3 70B Instruct

Strong quality candidate, but hardware and license make it conditional.

- 70B text-only instruct model.
- 128K context.
- Supports tool use formats.
- Custom Llama license, gated access, and large hardware footprint.

This is plausible for high-end local synthesis, not background defaults.

### OLMo 2 32B Instruct

Strong transparency candidate, but context is the blocker.

- Apache 2.0.
- AI2 provenance and unusually transparent release posture.
- 32B instruct model.
- Base model card lists 4K context, which is too small for many DailyOS use cases unless everything is chunked and retrieved carefully.

OLMo is interesting for research and transparency, but not the first model to productize for DailyOS.

### Cohere Command R7B

Interesting local RAG/tool model, but not product-default material because of licensing.

- 7B.
- 128K context.
- RAG and tool-use oriented.
- CC-BY-NC license in the model card.

Could be useful for personal/local experiments but should not be a default commercial DailyOS model unless licensing changes.

---

## Advanced BYOM Candidates

These should be supported through "bring your own model" because they may be excellent locally, but they are not the right privacy-first default.

### Qwen3-30B-A3B-Instruct-2507

- Apache 2.0.
- 30.5B total parameters, 3.3B active.
- 256K native context.
- Strong instruction following, long context, coding, and tool usage claims.
- Chinese-origin governance concern.

Local Qwen protects prompt privacy if inference stays on-device. The concern is provenance, supply chain, enterprise acceptability, and default-model optics.

### Qwen3-Coder-30B-A3B-Instruct

- Strong candidate for local coding/tool/agentic tasks.
- 256K native context.
- Same Chinese-origin governance concern.

Could be very useful for local action repair or developer-adjacent workflows, but should remain advanced/BYOM unless the product deliberately accepts the governance tradeoff.

### DeepSeek-R1 Distills

- MIT at the DeepSeek release level, with underlying base-model license caveats for distills.
- Strong reasoning family.
- Distilled sizes make local experiments practical.
- Chinese-origin risk is higher.
- Reasoning verbosity can be awkward for structured business workflows.

Useful for experiments and repair/reasoning jobs, but not a privacy-first default.

---

## Runtime Recommendation

DailyOS should not start by bundling a model runtime. It should first support detected local runtimes:

1. **Ollama** for the lowest-friction background service.
2. **LM Studio** for user-friendly model management, Apple MLX support, and local OpenAI-compatible APIs.
3. **OpenAI-compatible local HTTP provider** as the internal abstraction.
4. Later: managed/bundled runtime if product demand justifies app size, update, signing, and support burden.

The architecture should treat "OpenAI-compatible" as protocol, not privacy. An OpenAI-compatible endpoint can be local, self-hosted, LAN, cloud, or a third-party gateway. DailyOS needs an endpoint classifier:

- `LocalLoopback`: `localhost`, `127.0.0.1`, `::1`, Unix socket.
- `LocalManaged`: DailyOS-spawned child process with known binary/model path.
- `PrivateLan`: user-controlled network, not same-machine private.
- `SelfHostedRemote`: user-controlled server, remote inference.
- `CloudApi`: Anthropic/OpenAI/provider API.
- `CloudPty`: Claude Code, Codex, ChatGPT, or other CLI wrappers that call remote inference.

Only `LocalLoopback` and `LocalManaged` should qualify for "Local Private."

Local Private also needs a runtime egress policy. Ollama, LM Studio, and similar tools can be used privately, but they may also support cloud models, LAN serving, model search, update checks, or telemetry. DailyOS should treat these as separate from inference:

- Model download/search can use the network, but the user should initiate it.
- Inference requests for Local Private routes must go only to loopback or a DailyOS-managed process.
- Cloud models exposed by a local runtime do not qualify as Local Private.
- "Serve on LAN" settings should disqualify the route from Local Private unless the product introduces a separate private-network mode.
- Telemetry, update checks, and crash reporting must never include prompts, retrieved context, outputs, or embeddings.

---

## Recommended Test Matrix

The first evaluation batch should test:

1. `gpt-oss-20b` through Ollama.
2. `gpt-oss-20b` through LM Studio, ideally MLX on Apple Silicon.
3. Mistral Small 3.2 24B through LM Studio or vLLM.
4. Granite 4.0 H Tiny through a supported local runtime.
5. Phi-4 Mini ONNX or local runtime equivalent.
6. Gemma 3 12B IT.

Test against DailyOS tasks:

- Email relevance classification.
- Email action extraction with strict JSON.
- Meeting transcript action extraction with source refs.
- Entity enrichment from small context windows.
- Background hygiene suggestions.
- Repair/retry prompts for malformed JSON.
- Short executive summary of a bounded account context.

Gate on:

- JSON parse success.
- Schema validity.
- Source-ref preservation.
- Cross-entity bleed.
- Latency cold and warm.
- RAM/VRAM footprint.
- Battery/thermal behavior on MacBook-class hardware.
- Ability to run offline after model download.
- No network egress during inference.

---

## Product Recommendation

The v1.4.9 product framing should be:

> DailyOS supports multiple intelligence providers, including Claude Code, Anthropic API, OpenAI API, OpenAI-compatible endpoints, and a privacy-first local runtime. Local Private mode only uses downloaded models running on this machine and never silently falls back to cloud inference.

Implementation should split provider identity from execution location:

```rust
enum ProviderKind {
    ClaudeCode,
    Anthropic,
    OpenAI,
    OpenAICompatible,
    LocalRuntime,
    Other,
}

enum ExecutionLocation {
    LocalManaged,
    LocalLoopback,
    PrivateLan,
    SelfHostedRemote,
    CloudApi,
    CloudPty,
}
```

Then every model route can answer:

- Who provides the protocol?
- Which model is selected?
- Where does inference run?
- Is this route allowed for Local Private mode?
- Can it fall back, and to what?

For v1.4.9, I would test `gpt-oss-20b`, Mistral Small 3.2, Granite H Tiny, Phi-4 Mini, and Gemma 3 12B before deciding the default recommendation. My current default bet is:

1. **Default local recommendation:** `gpt-oss-20b`.
2. **Non-OpenAI local recommendation:** Mistral Small 3.2 24B.
3. **Low-resource/business-safe recommendation:** Granite 4.0 H Tiny or Phi-4 Mini after evals.
4. **Advanced BYOM:** Qwen3 and DeepSeek families.

---

## Sources Checked 2026-05-06

- OpenAI gpt-oss model card: https://openai.com/index/gpt-oss-model-card/
- OpenAI gpt-oss help center: https://help.openai.com/en/articles/11870455-openai-open-weight-models-gpt-oss
- OpenAI `gpt-oss-20b` Hugging Face model card: https://huggingface.co/openai/gpt-oss-20b
- OpenAI `gpt-oss-120b` Hugging Face model card: https://huggingface.co/openai/gpt-oss-120b
- OpenAI cookbook, gpt-oss with Ollama: https://developers.openai.com/cookbook/articles/gpt-oss/run-locally-ollama
- OpenAI cookbook, gpt-oss with LM Studio: https://developers.openai.com/cookbook/articles/gpt-oss/run-locally-lmstudio
- Ollama documentation: https://docs.ollama.com/
- LM Studio documentation: https://lmstudio.ai/docs
- Mistral Small 3.2 24B model card: https://huggingface.co/mistralai/Mistral-Small-3.2-24B-Instruct-2506
- IBM Granite 4.0 H Tiny model card: https://huggingface.co/ibm-granite/granite-4.0-h-tiny
- IBM Granite 4.0 H Small Base model card: https://huggingface.co/ibm-granite/granite-4.0-h-small-base
- Microsoft Phi-4 Mini ONNX model card: https://huggingface.co/microsoft/Phi-4-mini-instruct-onnx
- Microsoft Phi-4 Reasoning Plus model card: https://huggingface.co/microsoft/Phi-4-reasoning-plus
- Google Gemma 3 Hugging Face announcement: https://huggingface.co/blog/gemma3
- Google Gemma 3 12B IT model page: https://huggingface.co/google/gemma-3-12b-it
- Meta Llama 3.3 70B Instruct model card: https://huggingface.co/meta-llama/Llama-3.3-70B-Instruct
- AI2 OLMo 2 32B Instruct model card: https://huggingface.co/allenai/OLMo-2-0325-32B-Instruct
- Cohere Command R7B model card: https://huggingface.co/CohereLabs/c4ai-command-r7b-12-2024
- Qwen3-30B-A3B-Instruct-2507 model card: https://huggingface.co/Qwen/Qwen3-30B-A3B-Instruct-2507
- Qwen3-Coder-30B-A3B-Instruct model card: https://huggingface.co/Qwen/Qwen3-Coder-30B-A3B-Instruct
- DeepSeek-R1 model card: https://huggingface.co/deepseek-ai/DeepSeek-R1
