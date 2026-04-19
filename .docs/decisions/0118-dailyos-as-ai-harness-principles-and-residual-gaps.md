# ADR-0118: DailyOS as an AI Harness — Principles and Residual Gaps

**Status:** Proposed
**Date:** 2026-04-19
**Target:** Principles (v1.4.0 documentation) / Residual gap closure (v1.4.1+)
**Relates to:** [ADR-0080](0080-signal-intelligence-architecture.md), [ADR-0091](0091-intelligence-provider-abstraction.md), [ADR-0095](0095-dual-mode-context-architecture.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0107](0107-source-taxonomy-alignment.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0109](0109-temporal-primitives-in-the-entity-graph.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0112](0112-migration-strategy-parallel-run-and-cutover.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0114](0114-scoring-unification.md), [ADR-0115](0115-signal-granularity-audit.md), [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md)
**Linear:** [v1.4.0 — Abilities Runtime](https://linear.app/a8c/project/v140-abilities-runtime-8267614bba99), [DOS-5 Trust Compiler](https://linear.app/a8c/issue/DOS-5), [DOS-7 intelligence_claims persistence](https://linear.app/a8c/issue/DOS-7)

## Context

In 2025–2026 a design vocabulary crystallized around the idea of an **AI harness**: the runtime infrastructure surrounding an LLM that manages context assembly, tool dispatch, memory, verification, and observability. Anthropic's ["Effective harnesses for long-running agents"](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) (Nov 2025) and ["Harness design for long-running application development"](https://www.anthropic.com/engineering/harness-design-long-running-apps) (Mar 2026) formalized the pattern and advanced the thesis: **harness quality now dominates incremental model capability** for any task longer than a single context window. Most production agent failures in this era are harness bugs, not model bugs.

The useful layer distinction: **framework** (building blocks) < **scaffolding** (pre-prompt setup) < **orchestrator** (control flow) < **harness** (everything after the first token — capability surface, runtime, memory, verification).

DailyOS has been building a harness without naming it. The OpenClaw audit (`.docs/research/2026-02-14-openclaw-learnings.md`) was effectively a harness comparison done in different vocabulary. More importantly: the v1.4.0 Abilities Runtime project (ADRs 0102–0115) *is* the harness made explicit — typed capability contracts, structural provenance, mode-aware services, temporal primitives, trust scoring, evaluation harness, claims as first-class. The work already in flight is the answer. What has been missing is the framing.

This ADR does three things:

1. Adopts "AI harness" as the canonical framing for DailyOS's synthesis layer.
2. Maps harness best practices from the 2025–2026 literature onto existing ADRs, so contributors can see which harness principle each substrate decision satisfies.
3. Isolates the residual gaps — places where the harness literature points at something the v1.4.0 substrate does not fully address, with explicit scoping for each.

This ADR is principles + cross-reference + gap list. It does **not** propose new substrate. Where a gap needs closing, this ADR points at the existing ADR it should land in (amendment) or names it as a candidate future ADR.

## Decision

### 1. Framing: DailyOS is an AI harness

The term **harness** is adopted as canonical for DailyOS's synthesis and intelligence layer. Preferred over "pipeline," "agent," "enrichment system," or "intelligence engine" in architectural discussion. Rationale: *pipeline* undersells the memory and verification surface; *agent* oversells — DailyOS explicitly does not route across sub-agents, does not do recursive tool use, and does not run persistent background reasoning (see [ADR-0102](0102-abilities-as-runtime-contract.md) §1 and `.docs/research/2026-02-18-event-driven-intelligence-vision.md`). *Harness* is accurate: typed capability surface, durable external state, verification before output, feedback learning.

The v1.4.0 thesis stated in the project brief — "depth that compounds, surfaces that agree, trust that is computable" — is the harness thesis in DailyOS-specific language.

### 2. Harness principles and how the v1.4.0 substrate implements them

The table below maps ten principles distilled from Anthropic's harness posts and related practitioner writing onto the ADRs (present or in-flight) that satisfy them. Gaps flagged here are detailed in §3.

| Harness principle | DailyOS implementation |
|---|---|
| **P1 — Context engineering over context stuffing.** Dynamically select what the model sees per call; prefer smaller scoped prompts over monolithic ones. | Dimension split (I576), ability composition tree ([ADR-0102](0102-abilities-as-runtime-contract.md) §11.3), Glean-first retrieval ([ADR-0100](0100-glean-first-intelligence-architecture.md)), hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)). |
| **P2 — Durable external state as memory.** Memory lives in structured storage, not the context window. Each invocation is reconstructible from persisted state. | Signal event log ([ADR-0080](0080-signal-intelligence-architecture.md)), `intelligence_claims` table ([DOS-7](https://linear.app/a8c/issue/DOS-7)), temporal primitives ([ADR-0109](0109-temporal-primitives-in-the-entity-graph.md)), `chat_sessions` ([ADR-0075](0075-conversational-interface-architecture.md)), content embeddings ([ADR-0074](0074-vector-search-entity-content.md)). **Incomplete**: claim *history* (Gap C). |
| **P3 — Verification before trust.** Outputs are validated against deterministic ground truth before reaching the user; high-severity failures trigger repair, not user-visible error. | `validation.rs` (schema + anomaly), `consistency.rs` (fact-grounding), `TrustAssessment` in the Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md) §3), Trust Compiler six-factor score ([DOS-5](https://linear.app/a8c/issue/DOS-5)). |
| **P4 — Generator / evaluator separation.** Agents overrate their own output; an evaluator with an explicit rubric calibrates quality. Anthropic's v2 harness cites this as the single strongest lever. | Test-time: evaluation harness with LLM-as-judge rubrics ([ADR-0110](0110-evaluation-harness-for-abilities.md) §2, §5). **Incomplete at runtime** (Gap A). |
| **P5 — Subjective quality must be gradable.** Replace "is it good?" with scored rubric dimensions against calibrated exemplars. | Per-ability `quality.toml` rubric with relevance / faithfulness / attribution-completeness thresholds ([ADR-0110](0110-evaluation-harness-for-abilities.md) §5). |
| **P6 — Typed capability surface.** Tools / capabilities are versioned, schema-validated, category-classified with effect inference. | Abilities runtime contract — name, version, schema, category ([ADR-0102](0102-abilities-as-runtime-contract.md) §1–§3), service boundary enforcement ([ADR-0101](0101-service-boundary-enforcement.md)), mode-aware services ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)). |
| **P7 — Observability per invocation.** Trace tool calls, context contents, prompt, response, verification result, evaluator score — for every call. | Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md)), prompt fingerprinting ([ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)), invocation ID + inputs snapshot. **Incomplete**: debug UI consuming this data (Gap E). |
| **P8 — Model-provider abstraction.** Harness is the unit of improvement; swapping models must not require structural rewrites. | `IntelligenceProvider` trait ([ADR-0091](0091-intelligence-provider-abstraction.md)) amended with `Completion` + fingerprint metadata ([ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) §3), dual-mode context ([ADR-0095](0095-dual-mode-context-architecture.md)). |
| **P9 — Source attribution as output.** Every generated field carries attribution so corrections can flow back to the originating source's reliability weight. | `field_attributions` map in the Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md) §5), authoritative source taxonomy ([ADR-0107](0107-source-taxonomy-alignment.md)), provenance rendering + privacy ([ADR-0108](0108-provenance-rendering-and-privacy.md)). |
| **P10 — Feedback-learning loop.** User corrections update source reliability weights; confidence is earned, not declared. | Thompson Sampling over per-source Beta distributions ([ADR-0080](0080-signal-intelligence-architecture.md) Layer 3), `user_feedback_signals` (DOS-8 backend, v1.4.0), Trust Compiler feedback factor ([DOS-5](https://linear.app/a8c/issue/DOS-5)), scoring unification ([ADR-0114](0114-scoring-unification.md)). |

**Takeaway.** Nine of ten principles are substantively implemented by the v1.4.0 substrate or in-flight issues. P4 and P7 are partially implemented — each has a residual gap detailed below. P2 has a gap around claim *history*. No principle is absent. The v1.4.0 Abilities Runtime is not a set of scattered improvements — it is a harness, read in a particular vocabulary.

### 3. Residual gaps

Five gaps remain after v1.4.0 substrate lands. Listed by expected leverage.

#### Gap A — Runtime evaluator pass (highest leverage)

**What's missing.** [ADR-0110](0110-evaluation-harness-for-abilities.md) §5 defines a rubric-based LLM-as-judge scoring model *at test time*. At runtime, Transform abilities produce output that passes structural validation (schema, consistency, TrustAssessment) and reaches the user without subjective-quality check. Anthropic reports generator/evaluator separation is the strongest harness lever available; we have the evaluator at test time only.

**Proposal.** Add a lightweight runtime evaluator pass invoked after any Transform ability. Rubric reused from [ADR-0110](0110-evaluation-harness-for-abilities.md) §5 — same `quality.toml` definitions, same judge prompts, same fingerprinting. Low composite scores trigger re-prompt with evaluator critique attached as additional input. This is not a new rubric system; it is a second deployment surface for the one [ADR-0110](0110-evaluation-harness-for-abilities.md) already defines.

**Why it is not subsumed by ADR-0110.** ADR-0110 operates offline on fixtures to gate merges. It cannot see production inputs and does not run on live invocations. Runtime evaluation sees the actual user context and can repair before the user ever sees the bad output.

**Dependencies.** [ADR-0102](0102-abilities-as-runtime-contract.md) (invocation hook on Transform ability completion), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) (evaluator's own prompt is fingerprinted), [ADR-0110](0110-evaluation-harness-for-abilities.md) (rubric source of truth).

**Scope.** Research spike within the v1.4.0 enrichment-refactor spike (see §5). Implementation target: v1.5.0. Cost concern — runtime evaluation doubles LLM calls on Transform paths; mitigate with sampling (evaluate every invocation on failure-suspected abilities, sample 10–20% elsewhere) or cheaper judge model.

**Status:** Resolved by [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) (Runtime Evaluator Pass for Transform Abilities). ADR-0119 specifies the hook, rubric reuse, sampling, retry-with-critique, trace storage, and mode awareness. Implementation target v1.5.0 with enrichment as first opt-in ability.

#### Gap B — Harness-stripping evaluation

**What's missing.** Anthropic's harness posts warn that "every harness component encodes an assumption about what the model can't do alone," and those assumptions go stale. [ADR-0110](0110-evaluation-harness-for-abilities.md) measures ability output quality but does not periodically re-test whether harness components still earn their keep on current models. Dimension split, PTY concurrency limits, consistency-check retry budgets, and the dimension fan-out itself are all such assumptions.

**Proposal.** Add a new fixture class to [ADR-0110](0110-evaluation-harness-for-abilities.md): *stripped-harness fixtures*. Same input state and expected output as a standard fixture, but with named harness components disabled (e.g., `--strip=dimension_split`, `--strip=consistency_repair`). Run quarterly on a curated set of goldens. Any harness component that no longer loses to the stripped variant on current models becomes a candidate for retirement.

**Dependencies.** [ADR-0110](0110-evaluation-harness-for-abilities.md) extension (new `strip` field on fixture manifest), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) (fingerprint difference is the class signal).

**Scope.** Amendment to [ADR-0110](0110-evaluation-harness-for-abilities.md) post-v1.4.0. Low urgency until at least one harness component falls under suspicion, but cheap to instrument once the eval harness is operational.

**Status:** Resolved by [ADR-0110](0110-evaluation-harness-for-abilities.md) §9 (Harness-Stripping Fixtures) amendment. Specifies the fixture manifest extension, strip-delta scoring, quarterly cadence, governance for adding new components to the strip set. Activation follows the eval harness rollout in v1.4.0.

#### Gap C — Claim history (the harness's episodic memory of itself)

**What's missing.** The `intelligence_claims` schema in [DOS-7](https://linear.app/a8c/issue/DOS-7) captures claims with `trust_score`, `trust_computed_at`, `trust_version`. The schema as drafted appears to support *trust* versioning (the score changed) but not *assertion* versioning (the claim itself changed or was retracted). Without assertion history we cannot:

- Diff run N vs. run N−1 to see what the AI changed its mind about.
- Use stability-as-confidence — a claim repeated across 10 runs is more credible than a fresh one, but today they look identical.
- Detect flapping (claim oscillating between values) as a reliability signal for the underlying source.
- Feed "has this assertion been stable" as a rubric dimension into the runtime evaluator (Gap A).

This is distinct from [ADR-0109](0109-temporal-primitives-in-the-entity-graph.md). Temporal primitives store *derived trajectories* over structured data (health, engagement, role progression). Claim history stores *the AI's own assertions about those things*, which is what lets us catch drift in the AI, not just in the underlying entity.

Concrete user case documented during v1.0.0 GA: users remove a value from a multi-select (e.g., a role type); because enrichment is ephemeral and the AI does not see the prior state or the removal, the next run repopulates the field. The current workaround is to stub a sentinel value so the prompt has something to preserve. This is a symptom of two distinct missing primitives: claim history (Gap C) and negative-knowledge tombstones (Gap D).

**Proposal.** During [DOS-7](https://linear.app/a8c/issue/DOS-7) design review, specify `intelligence_claims` as append-only with respect to assertion changes. Concretely: add `claim_sequence` (monotonic per `(entity_id, claim_type, field_path)`), `previous_claim_id` (nullable FK), `superseded_at` (nullable), `superseded_by` (nullable FK), `retraction_reason` (nullable enum). Keep the drafted `trust_version` for trust-only revisions. Prune policy: retain last N assertion values per field plus aggregate stability metric (flap rate, assertion-age, distinct-value count). Bounded growth — under 10K entities × ~20 claim types × ~1 change/week, pruned history fits well under 10M rows.

**Why append-only.** Trust is computable from any point in history ([DOS-5](https://linear.app/a8c/issue/DOS-5)); the only cost of append-only is storage. The benefit — harness episodic memory — is load-bearing for Gaps A and B.

**Dependencies.** [DOS-7](https://linear.app/a8c/issue/DOS-7) (the right place to decide this), [ADR-0105](0105-provenance-as-first-class-output.md) (`ProvenanceWarning::SourceStale` can extend to claim-stale), [ADR-0110](0110-evaluation-harness-for-abilities.md) (stability rubric dimension), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) (propose/commit semantics).

**Scope.** Design decision during [DOS-7](https://linear.app/a8c/issue/DOS-7) review. This ADR recommends append-only as the default; the alternative (mutate-in-place + separate audit log) is materially worse for harness episodic memory.

**Status:** Resolved. [DOS-7](https://linear.app/a8c/issue/DOS-7) schema now specifies append-only assertions via `previous_claim_id` + `superseded_at` + `superseded_by` + `claim_sequence`. [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) R1.6 adds `claim_corroborations` child table so per-asserter dedup preserves history. [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) §9 adds stability-as-confidence as an opt-in evaluator dimension that consumes claim history. Trust-score history remains deferred — the compiler is deterministic and recomputable from history; a `claim_trust_history` table will be added as a follow-on only if audit demand materializes.

#### Gap D — Negative knowledge / user-intent tombstones

**What's missing.** The database cannot distinguish between "field is absent because not yet known" (AI should fill) and "field is absent because the user intentionally removed it" (AI should not repopulate). This produces the ghost-resurrection bug described in Gap C.

**Proposal.** Introduce tombstone claims: when a user removes a value, record a Human-source claim of absence with a `retraction_reason` and `retracted_by_actor = User`. Enrichment prompts and Transform abilities read tombstone claims as hard constraints: the prompt must be told the user rejected this value, and the ability must not repopulate unless evidence exceeds a configurable threshold.

**Where this lands.** [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) §5 now specifies tombstones concretely: `actor = 'user'` or `user_removal`, `claim_text = NULL`, `retraction_reason = 'user_removal'`, `claim_state = 'committed'`. §3's commit policy gate includes a hard check against recent tombstones at the target `field_path` — an agent claim targeting a tombstoned field within the tombstone window (default 30 days) is rejected before it enters `proposed` state. Gap D is subsumed.

**Residual risk.** The tombstone window default (30 days) and the corroboration threshold for overriding a tombstone are tuning parameters that will be validated in the v1.4.1 enrichment shadow. If shadow data shows legitimate re-population being blocked (e.g., a stakeholder correctly returning after a temporary absence), tuning happens there.

**Scope.** Closed by [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md). Monitoring only in v1.4.1.

#### Gap E — Debug trace surface

**What's missing.** Provenance envelopes ([ADR-0105](0105-provenance-as-first-class-output.md)), prompt fingerprints ([ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)), and the eval harness ([ADR-0110](0110-evaluation-harness-for-abilities.md)) collectively produce all the data needed to answer "why did the AI say this" — assembled context, prompt text, response, validation flags, consistency report, trust score, composition tree. No UI consumes this data. Debugging a bad output today requires stitching logs by hand.

**Proposal.** A devtools-style panel on the entity page (dev mode only, gated by [ADR-0108](0108-provenance-rendering-and-privacy.md) privacy rules) showing the last N invocations per entity: context hash + resolvable pointer, prompt fingerprint + resolvable template, provider response, validation + consistency + trust output, composition tree. Not new storage — a new consumer of existing storage.

**Dependencies.** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0108](0108-provenance-rendering-and-privacy.md) (the panel must honor masking rules and privacy tiers), [ADR-0111](0111-surface-independent-ability-invocation.md) (panel is a Surface per ADR-0111's definition; use the invocation endpoint, not a bespoke query).

**Scope.** v1.5.0 or later. Not substrate-blocking; pure product surface work.

**Status:** Tracked as Linear issue for v1.5.0+ product work. Substantively empowered by the addition of `evaluation_traces` table in [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) §6, which gives the panel rich data beyond provenance alone. No new ADR required.

---

### Gap resolution summary

| Gap | Status | Resolved by |
|---|---|---|
| A — Runtime evaluator pass | Resolved | [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) |
| B — Harness-stripping evaluation | Resolved | [ADR-0110](0110-evaluation-harness-for-abilities.md) §9 amendment |
| C — Claim history | Resolved | [DOS-7](https://linear.app/a8c/issue/DOS-7) schema + [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) R1.6 + [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) §9 stability dimension |
| D — Tombstones / negative knowledge | Resolved | [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) §5 + R1.1 + R1.3 (consolidates existing `suppression_tombstones`, `DismissedItem`, `account_stakeholder_roles.dismissed_at`) |
| E — Debug trace surface | Tracked as v1.5.0+ product work | Linear issue (filed 2026-04-19); consumes `evaluation_traces` + provenance |

All residual gaps identified in the original ADR either closed by specific ADR work or tracked as product-surface issues with substrate dependencies satisfied.

### 4. What is explicitly *not* a gap

- **Persistent background reasoning / always-on AI.** Rejected in `.docs/research/2026-02-18-event-driven-intelligence-vision.md`. The v1.4.0 three-minds pattern (Scheduler / Signal Engine / Reasoning Layer) satisfies the "always-on" user promise through good retrieval + signal fusion, not long-running processes.
- **Sub-agent orchestration.** The dimension split and ability composition tree look like parallel sub-agents but are not — dimensions and composed abilities do not communicate, plan, or invoke each other laterally. Introducing true lateral agents would invert the harness thesis ("LLM produces, users consume") and is out of scope.
- **`init.sh`-style session bootstrap.** Anthropic's pattern targets multi-session coding agents. DailyOS sessions are per-ability invocations; cross-session continuity is served by the claims layer + temporal primitives + briefing callouts. No additional bootstrap artifact is needed.
- **Novel eval rubric system.** Gap A deliberately reuses [ADR-0110](0110-evaluation-harness-for-abilities.md)'s rubric. Anything requiring a parallel quality definition is rejected.

### 5. Input into the v1.4.0 enrichment-refactor research spike

The v1.4.0 project mandates a research spike producing a design doc that maps current enrichment pipelines onto abilities, integrated with trust scoring, the propose/commit boundary ([ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md)), reversibility (DOS-12), outbox ([ADR-0103](0103-maintenance-ability-safety-constraints.md)), and evaluation harness ([ADR-0110](0110-evaluation-harness-for-abilities.md)). This ADR amends that mandate. The spike should additionally address:

1. Whether enrichment produces claims append-only or mutates in place (Gap C — this ADR recommends append-only).
2. How enrichment consumes user tombstone claims to prevent ghost resurrection (Gap D).
3. Whether a runtime evaluator pass (Gap A) gates enrichment output before write, and if so the sampling + cost policy.
4. How enrichment observability (Gap E) surfaces in a developer-facing trace panel during debugging.

## Consequences

### Positive

- Shared vocabulary. "Is this a harness change?" becomes answerable. Contributors can map any proposed change to one of the ten principles in §2.
- Explicit cross-reference. The principle → ADR mapping reduces "do we already do this?" ambiguity during review and onboarding.
- Five residual gaps named and scoped. Two of them (C, D) become concrete design inputs into [DOS-7](https://linear.app/a8c/issue/DOS-7) and [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) before those ship — the cheapest time to decide them.
- Alignment with industry pattern. DailyOS's synthesis layer is legible to practitioners using the 2025–2026 vocabulary, which matters for hiring, partnership conversations, and the architecture-as-marketing surface.

### Negative / risks

- "Harness" is recent jargon. It may drift by 2027. Revisit vocabulary annually; this ADR owns that review.
- Gap A (runtime evaluator) doubles LLM call count on Transform paths. Sampling strategy and cost target must be settled during the v1.4.0 enrichment spike.
- Gap C (append-only claims) is bounded-growth but requires a pruning policy. The work of defining that policy lands on [DOS-7](https://linear.app/a8c/issue/DOS-7) review.
- Risk of over-applying harness framing to code that does not benefit. The ten principles in §2 are descriptive of what works; they are not a mandate to refactor code that already ships.

### Neutral

- This ADR adds no runtime code. Principles + cross-reference + gap list only.
- Gap closure is distributed: A goes into the enrichment spike + v1.5.0; B is a [ADR-0110](0110-evaluation-harness-for-abilities.md) amendment; C is a [DOS-7](https://linear.app/a8c/issue/DOS-7) design decision; D is input into [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md); E is a v1.5.0+ product surface. None require a new foundational ADR.

## References

External (the harness literature):

- [Anthropic — Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) (Nov 2025)
- [Anthropic — Harness design for long-running application development](https://www.anthropic.com/engineering/harness-design-long-running-apps) (Mar 2026)
- [Parallel — What is an agent harness?](https://parallel.ai/articles/what-is-an-agent-harness)
- [LangChain — Your harness, your memory](https://www.langchain.com/blog/your-harness-your-memory)
- [Cobus Greyling — The rise of AI harness engineering](https://cobusgreyling.medium.com/the-rise-of-ai-harness-engineering-5f5220de393e)
- [Epsilla — GAN-style agent loop: deconstructing Anthropic's harness](https://www.epsilla.com/blogs/anthropic-harness-engineering-multi-agent-gan-architecture)
- [InfoQ — Anthropic's three-agent harness](https://www.infoq.com/news/2026/04/anthropic-three-agent-harness-ai/)
- [Sebastian Raschka — Components of a coding agent](https://magazine.sebastianraschka.com/p/components-of-a-coding-agent)

Internal:

- `.docs/research/2026-02-14-openclaw-learnings.md` — prior OpenClaw harness audit
- `.docs/research/2026-02-18-event-driven-intelligence-vision.md` — three-minds architecture
- [v1.4.0 — Abilities Runtime](https://linear.app/a8c/project/v140-abilities-runtime-8267614bba99) project brief
- [DOS-7 — intelligence_claims persistence](https://linear.app/a8c/issue/DOS-7)
- [DOS-5 — Trust Compiler scoring core](https://linear.app/a8c/issue/DOS-5)
