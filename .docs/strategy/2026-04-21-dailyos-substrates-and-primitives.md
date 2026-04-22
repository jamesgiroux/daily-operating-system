# DailyOS: Substrates and Primitives for Personal Intelligence

**Date:** 2026-04-21. **Author:** James Giroux.
**Status:** Foundational. Supersedes the scattered strategic notes written earlier this week.

---

## What this is

DailyOS started as a personal productivity project and has become something else along the way. It is the vehicle through which I am exploring, in a greenfield codebase with no installed-base constraints, the hard problems every AI-for-work tool is hitting right now: trust, provenance, accuracy, context, persistent memory, correction durability, privacy at scale.

Everyone in this category is learning the same lessons at roughly the same time. Karpathy published an LLM Wiki gist last week. Garry Tan's GBrain shipped for himself. OpenClaw and Hermes are building harnesses for engineer audiences. Across Automattic, teams are building their own versions of this thing. All of them are running into the same walls.

The walls are not about the model. The walls are about what you build around the model. Memory, trust, provenance, abilities, self-healing. That is the harness. The harness is what determines whether an AI becomes a tool you depend on every day or a clever demo you open twice and forget.

DailyOS is roughly six months ahead of the public conversation on how the harness should work, because I have been building it for six months and publishing nothing. That is the lead-not-follow opportunity this document argues for.

## How we got here

The `.docs/` directory is the whole story if you read it in order. The short version:

**January 2024.** First architectural decision ([ADR-0001](../decisions/0001-use-tauri-over-electron.md)): Tauri over Electron. Native binary, Rust backend, React frontend, small footprint. The priorities that would define everything else were already in the first paragraph: local, fast, system-integrated, AI-subprocess-ready.

**The founding triad.** Before most of the ADRs, three design documents established the values: [VISION.md](../../design/VISION.md) ("Open the app. Your day is ready."), [PHILOSOPHY.md](../../design/PHILOSOPHY.md) ("Your brain shouldn't have a landlord"), and [PRINCIPLES.md](../../design/PRINCIPLES.md) ("The system operates. You leverage."). Everything that follows is the working-out of these three documents under real engineering pressure. They have not changed. That is unusual, and it is what keeps the system coherent as it grows.

**ADRs 0001-0030 (early 2024 to mid 2025).** UI and workflow decisions. Profile switching, sidebar structure, meeting drill-down, hybrid markdown-plus-SQLite storage ([ADR-0018](../decisions/0018-hybrid-storage-markdown-sqlite.md)), structured document schemas ([ADR-0028](../decisions/0028-structured-document-schemas.md)), MCP dual-mode ([ADR-0027](../decisions/0027-mcp-dual-mode.md)). Productivity-tool shape, with the AI layer still relatively thin.

**[ADR-0006](../decisions/0006-determinism-boundary.md), February 2026.** "Phase 3 generates JSON (determinism boundary)." Two months ago, in plain language, the architectural split this document names: "Phase 1 and 3 are deterministic Python; Phase 2 is non-deterministic AI." The insight was baked in early. Every ADR since has been a more detailed answer to the same question: where does code end and AI begin, and how do we contract between them.

**ADRs 0030-0099 (2025 through early 2026).** Proactive meeting research ([ADR-0022](../decisions/0022-proactive-research-unknown-meetings.md)), three-tier email priority ([ADR-0029](../decisions/0029-three-tier-email-priority.md)), signal intelligence architecture ([ADR-0080](../decisions/0080-signal-intelligence-architecture.md)), intelligence provider abstraction ([ADR-0091](../decisions/0091-intelligence-provider-abstraction.md)), encryption at rest ([ADR-0092](../decisions/0092-data-security-at-rest-and-operational-hardening.md)). The AI substrate started taking shape.

**ADRs 0100-0121, April 2026.** The substrate formalisation. Service boundary enforcement ([0101](../decisions/0101-service-boundary-enforcement.md)), abilities runtime contract ([0102](../decisions/0102-abilities-as-runtime-contract.md)), execution mode and mode-aware services ([0104](../decisions/0104-execution-mode-and-mode-aware-services.md)), provenance as first-class output ([0105](../decisions/0105-provenance-as-first-class-output.md)), human and agent claims ([0113](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md)), scoring unification ([0114](../decisions/0114-scoring-unification.md)), signal granularity ([0115](../decisions/0115-signal-granularity-audit.md)), tenant control plane boundary ([0116](../decisions/0116-tenant-control-plane-boundary.md)), publish protocol ([0117](../decisions/0117-publish-boundary-pencil-and-pen.md)), AI harness principles ([0118](../decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)), runtime evaluator ([0119](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md)), observability contract ([0120](../decisions/0120-observability-contract.md)). The primitives catalogued below are the working surface of this layer.

The through-line is continuity. Philosophy and principles did not move. Each ADR inherits the values of the founding triad and applies them to a specific engineering problem. What looks like six months of sprinting is actually two years of compounding.

## What DailyOS actually is

A native macOS app (Tauri + Rust + React), single-user, local-first, encrypted at rest. On the surface it is an AI chief of staff that turns the raw stream of a knowledge worker's day (calendar, email, transcripts, CRM, Glean search) into a trusted daily briefing.

Under the surface it is something more important: a working reference implementation of a harness for AI-native work, built by a company whose values force the hard architectural choices no open-source engineer project has taken seriously yet. Local-first because ownership. Provenance because trust. Tombstones because user corrections matter. Privacy boundary because the moment you index personal signals to a server, the honest signals stop being written.

The six-month head start is not in clean code or great UI. The code is uneven in places and the UI is functional, not polished. The head start is in the substrate: the set of primitives that handle the hard parts of making AI trustworthy. Those primitives are what I think we take to the rest of the company.

## The primitives

Each of these is real in DailyOS today or landing as part of v1.4.0. Each is decoupled enough from DailyOS specifics to be extracted and applied to another product.

### 1. Persistent memory

**What it is.** A structured, append-only ledger of claims (facts about entities) with supersede pointers, tombstones, and per-claim provenance. Stored in encrypted SQLite. Indexed for trust-weighted retrieval, full-text search, and vector recall.

**What it does.** It survives between sessions, LLM calls, restarts, even LLM provider swaps. The claim "Alice is the champion at Acme" is written once, trust-scored, provenanced, linked to the meeting transcript it came from, and never regenerated from scratch. When Alice leaves Acme, a new claim supersedes the old; the history remains.

**Where it lives.** ADR-0113 (claim ledger), ADR-0107 (source taxonomy), ADR-0105 (provenance envelope).

**Why other products want it.** Every AI-for-work product is rebuilding this in a worse form. A Salesforce AI that summarises an account forgets its summary the next time. A meeting prep tool regenerates context from zero every invocation. Persistent memory, as a shared primitive, means every Automattic product's AI gets better over time at knowing the things it already learned.

### 2. Trust scoring

**What it is.** Every claim compiles a trust score from six factors (source reliability, freshness, corroboration, contradiction, user feedback, meeting relevance). The score maps to a visible band: `likely_current`, `use_with_caution`, `needs_verification`.

**What it does.** It makes the AI's confidence legible to the user and to downstream systems. A Transform ability asked to draft a meeting brief can filter to high-trust claims only. A UI can render trust bands as visual signals. A user correction can flow back into the compiler as feedback, raising or lowering the weight on the source that produced the claim.

**Where it lives.** ADR-0114 (scoring unification), ADR-0110 (evaluation harness), DOS-5 (trust compiler implementation).

**Why other products want it.** Every AI output today shows up with the same visual weight regardless of how confident it should be. Trust scoring as a primitive means every AI surface can communicate uncertainty honestly without every team having to design its own confidence model from first principles.

### 3. Provenance

**What it is.** Every ability output carries a provenance envelope with identity, temporal context, source attribution, composition tree, field-level attribution, and prompt fingerprint. Hard-capped at 64 KB serialised. Surfaces render a click-to-source affordance over any claim.

**What it does.** It makes "where did this come from" a one-click answer. No opaque model output. Every fact in a briefing resolves to a transcript line or an email thread with a timestamp. Every synthesised paragraph identifies which source claims contributed which fragments.

**Where it lives.** ADR-0105 (provenance as first-class output), ADR-0106 (prompt fingerprinting), ADR-0108 (rendering and privacy).

**Why other products want it.** Regulated industries (healthcare, finance, legal) require this by law. Enterprise buyers require it by policy. Consumers are starting to require it by taste. A product that cannot answer "why did you say that" is a product that will not last. Provenance as a primitive is table stakes, and nobody ships it well yet.

### 4. Self-healing

**What it is.** Two pieces. A runtime evaluator (ADR-0119) that scores Transform outputs against a rubric before they reach the user. A lint mode (DOS-274) that surfaces contradictions, stale high-confidence claims, orphan entities, and superseded-but-referenced content for user review or auto-resolution.

**What it does.** It closes the loop between "the AI generated something" and "the AI generated something that holds up over time." Bad outputs get flagged before display. Drifted claims get re-verified on a schedule. Contradictions surface instead of compounding.

**Where it lives.** ADR-0119 (runtime evaluator), ADR-0110 (eval harness), DOS-274 (lint mode).

**Why other products want it.** Every AI product accumulates error over time if the only correction path is the user noticing and flagging it. Self-healing primitives turn silent rot into visible maintenance.

### 5. The intelligence loop

**What it is.** The contract that ties everything else together. Sources produce raw signals. Signals propose claims. Claims get trust-scored, corroborated, or contradicted. High-confidence claims get committed to persistent memory. Claims feed Transform abilities that generate outputs. Outputs get runtime-evaluated. User feedback flows back as trust-score adjustments and tombstones.

**What it does.** It is the circulatory system. Every piece of AI work in DailyOS moves through this loop. It is what makes the system "already know" instead of "start from zero every prompt."

**Where it lives.** CLAUDE.md ("Intelligence Loop integration check"), the full ADR stack composed. Every new table, every new column, every new data surface has to answer five questions about how it participates in this loop before it ships.

**Why other products want it.** Every AI product needs some version of this. Most are building ad-hoc pipelines where signals, claims, and outputs get tangled together in ways that make debugging impossible. A shared intelligence-loop primitive gives every team a ready-made shape to build against.

### 6. Typed abilities

**What it is.** Every AI capability (`prepare_meeting`, `get_entity_context`, `detect_risk_shift`) is a named, typed, versioned function in one of four categories: Read, Transform, Publish, Maintenance. All surfaces (Tauri UI, MCP server, background workers) invoke through one registry. Mandatory provenance on output.

**What it does.** It turns AI capabilities into a composable, auditable surface. Any client (UI, MCP, another product) can list the abilities, call them with typed arguments, and receive typed outputs with provenance attached. No bespoke command handlers. No LLM direct-call from surface code.

**Where it lives.** ADR-0102 (abilities as runtime contract), ADR-0111 (surface-independent invocation).

**Why other products want it.** The moment a product has three AI features, the ad-hoc approach stops scaling. Typed abilities as a primitive let a product team add the tenth AI feature with the same discipline as the first. Also: MCP-ready from day one, which matters as the agent-exposure story matures.

### 7. Privacy boundary

**What it is.** The architectural commitment that no content ever leaves the user's machine (ADR-0116). Server-side components see metadata only: identity, capability grants, aggregate telemetry. LLM calls use user-owned keys (BYO-key). Local storage is encrypted at rest (ADR-0092).

**What it does.** It makes "your brain shouldn't have a landlord" structural. Not a setting. Not a promise. An architectural guarantee backed by the absence of a place for content to leak.

**Where it lives.** ADR-0092 (encryption at rest), ADR-0116 (tenant control plane boundary).

**Why other products want it.** This is the single most defensible positioning Automattic has in the AI category. Microsoft and Google cannot make this claim. Notion and SuperHuman cannot make this claim. OpenAI cannot make this claim. Automattic can, and we can make it credible because the architecture actually forces the property.

### 8. Durable corrections

**What it is.** When a user rejects a claim, a tombstone records the intent. The tombstone check is pre-gate: it runs before any enrichment cycle attempts to commit a new claim in the same slot. No agent can silently repopulate a tombstoned value. The ledger is append-only, so the tombstone history is auditable forever.

**What it does.** It kills ghost-resurrection. This is the bug that destroys every AI assistant people have tried. The user corrects something, the AI forgets, the correction quietly reverts, trust collapses. Tombstones plus pre-gate make that failure structurally impossible.

**Where it lives.** ADR-0113 (claim ledger, tombstone R2 amendment), ADR-0115 (signal propagation respect).

**Why other products want it.** Same reason as above. Every AI product needs user corrections to stick. None of them get it right today.

## The deterministic / probabilistic boundary

The term you were reaching for is probabilistic. Deterministic code means same input, same output, always. Probabilistic in this context means LLM-generated: same input can produce different outputs because sampling is stochastic.

The hardest architectural question in an AI harness is where to draw the line between the two. DailyOS's answer, accumulated across the ADRs, is this:

### Deterministic (code you can reason about, test exhaustively, audit)

- Claim storage (insert, supersede, tombstone, retrieve)
- Trust score computation (pure function of six inputs)
- Signal propagation (exhaustive compile-time-checked policy registry)
- Tombstone pre-gate enforcement (one `WHERE NOT EXISTS` query)
- Invalidation queue (jobs with idempotency keys)
- Provenance envelope assembly (structured, no free text)
- Structured link extraction (frontmatter-driven, no LLM)
- Lint queries (SQL over the claim ledger)
- Ability dispatch (registry lookup)
- Privacy boundary enforcement (no server-side content path exists)

These are the parts where "correct" is a verifiable property. Tests exist. Bugs are findable. Proofs are possible.

### Probabilistic (LLM-generated, sampled, inherently variable)

- Transform abilities (the content of `prepare_meeting`, `draft_followup`)
- Unstructured extraction (prose → candidate claims)
- Natural-language summarisation
- Runtime evaluator scoring (LLM-as-judge rubric)
- Name resolution fallbacks (when deterministic rules are ambiguous)

These are the parts where "correct" is a judgment call. You can improve the average outcome. You cannot guarantee a specific output. Testing is statistical, not exhaustive.

### The contract between them

This is the architectural insight. Probabilistic outputs never commit directly to state. They produce candidate claims that enter the deterministic substrate, where the determinic system applies policy: trust gates, tombstone pre-gates, category enforcement, provenance validation, size caps.

The probabilistic layer is powerful. The deterministic layer is safe. The contract between them is what makes the harness trustworthy: probabilistic work is free to fail, because deterministic work catches the failures before they become user-visible lies.

Every new feature faces the same design question: which side of the line is this, and if it crosses, what is the contract? The ADRs are the accumulated answers.

## Why primitives, not a product

For a long time I have been pitching DailyOS as the product. The commercial lens note from earlier today argued for work-as-wedge and personal-as-moat, which made DailyOS Pro the revenue line. That's still plausible, but I have changed my mind about the RSM framing.

The RSM month should not be "make DailyOS shippable." The RSM month should be "extract and harden the substrate primitives such that they can be adopted by another Automattic product team, and start the conversation with at least one."

The reason is about where the leverage is. DailyOS as a standalone product is one team's bet. DailyOS's substrate as shared infrastructure is every Automattic AI team's force multiplier. The same substrate that makes DailyOS trustworthy can make Jetpack AI trustworthy, WooCommerce admin AI trustworthy, VIP operational tooling trustworthy, Beeper's summarisation trustworthy, me.sh's depth real instead of one-dimensional.

I can build the flagship consumer of the substrate. I cannot build ten consumers. Automattic's AI story rides on whether every product team gets a head start on the hard parts or has to re-derive them from scratch.

## How the primitives apply across Automattic

Concrete, not aspirational. A first pass at how each primitive lands in existing products:

| Primitive | Jetpack AI | WooCommerce admin AI | Beeper | me.sh | Day One | Gravatar |
|-----------|------------|---------------------|--------|-------|---------|----------|
| Persistent memory | Cross-post claim ledger per site | Product / order / customer entity graph | Message summary ledger | The people-ledger they need | Entry-to-entity graph | Canonical identity provenance |
| Trust scoring | Confidence bands on AI suggestions | High-trust vs verify-first on operational recommendations | Summary confidence per thread | Relationship claims with confidence | Entry-level trust on auto-extracted tags | Verification state per profile field |
| Provenance | Click-to-see-why on every AI suggestion | Source-data trail for every operational insight | Link summary to source messages | Click to see the thread a note came from | Photos, locations, notes as sources for extracted claims | Which source confirmed which profile field |
| Self-healing | Stale recommendation detection | Contradicted operational claim flagging | Drift detection on personal context | Contradiction surfacing in relationship history | Timeline contradictions over years | Obsolete field detection |
| Intelligence loop | Yes, per site | Yes, per merchant | Yes, per person | Yes, per contact | Yes, per journal | Yes, per identity |
| Typed abilities | Replace ad-hoc AI commands | Replace ad-hoc AI commands | Replace ad-hoc AI commands | Replace the whole backend | Gradually adopt | Light adoption |
| Privacy boundary | Already partial, formalise | New, critical | Already core | New, critical | Already core | Already core |
| Durable corrections | User fixes an AI suggestion, it stays fixed | Merchant override persists | User corrects a summary, it stays | User corrects a relationship, it stays | User corrects an extracted claim, it stays | User corrects a profile field, it stays |

This is where "personal intelligence as a persistent layer across products" becomes concrete. The layer is the primitives. The products stay themselves.

## The RSM laboratory

With the reframe, the month's goal sharpens:

1. **Ship the substrate end-to-end in DailyOS.** Two real abilities (`get_entity_context`, `prepare_meeting`) running on the v1.4.0 substrate. Daily driver for me. Stable enough that at least one curious Automattician can install it and have it not break.
2. **Extract the primitives into documentation.** Each of the eight primitives gets a one-pager: what it is, what it solves, what the minimum implementation looks like, what it costs to adopt. Public-facing quality.
3. **Open at least one conversation with another product team.** Not an adoption commitment. A thirty-minute meeting to see if the primitives map to a real AI bottleneck the team has. me.sh is the obvious first conversation given the incoming meeting request. Jetpack AI or WooCommerce admin AI would be close seconds.
4. **Publish something.** An internal P2 describing the substrate. A public post (later, not during the month) positioning Automattic in the harness-ahead-of-model conversation. Engagement with the Karpathy gist comment thread with specific solutions to specific problems, grounded in our ADRs.

The month's deliverable is not a product. It is proof that the substrate is real, applicable beyond DailyOS, and worth the organisation's attention.

## The opportunity to lead

Every engineer-facing tool in this space (GBrain, LLM Wiki, OpenClaw, Hermes) is roughly where we were six months ago. They are running into trust, provenance, correction durability, and memory problems. Their solutions are ad-hoc. The comment thread on Karpathy's gist is a greatest-hits list of "we haven't solved that yet."

We have solved it. Most of it. Not by being smarter, by being earlier and more disciplined. Each time one of those problems bit us, we wrote an ADR, reviewed it, hardened it, implemented it. The accumulated work is not code cleanliness or UI polish. It is answers to the hard questions, written down, tested, shipped.

The window to publish is open. If we wait another six months, the engineer-facing tools will have caught up, and "Automattic has been thinking about this longer" will not be news. If we publish now (not a sales pitch, a substrate walkthrough with the ADRs as evidence), we become a reference point in a conversation where nobody is yet the reference point.

This is the lead-not-follow move. It is bounded (the substrate exists, the writing is the new work). It is on-brand (open, user-owned, disciplined). It aligns with the category where Automattic is structurally the only credible candidate. It gives the organisation an outside signal that the internal investment is real.

## What I want out of the next month

1. Ship the substrate end-to-end on two real DailyOS abilities, running as my daily driver.
2. Write the eight primitive one-pagers. Public-quality documentation that another product team could adopt from.
3. Take the me.sh meeting. Take one other product-team conversation (Jetpack AI or WooCommerce AI, most likely).
4. Publish an internal P2 on the substrate. Decide with a founder whether and when to publish externally.
5. End the month with a clear organisational decision: is the substrate a shared primitive layer for Automattic's AI products, or is it a DailyOS-only thing. Either answer is fine; the decision is the output.

I do not need headcount. I do not need budget. I need one or two RSM partners, one architectural review slot mid-month, and the space to write.

## Close

DailyOS is not necessarily the product we release. It is the vehicle for learning, in public, how to do AI-native work at the level of discipline Automattic's values require. The primitives DailyOS has produced are the most important output. Making them legible, adoptable, and visible to the rest of the organisation and the rest of the industry is how we lead a category instead of following it.

The substrate exists. The time to publish it is now.
