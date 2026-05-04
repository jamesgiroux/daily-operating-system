# ADR-0128 — Headless DailyOS: MCP as a co-equal product surface

**Status:** Accepted
**Date:** 2026-05-03
**Amended:** 2026-05-04 — added §7 (CLI as a third head, mediated by MCP)
**Authors:** James Giroux, Claude
**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)
**Linear:** [v1.5.0 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v150-mcp-server-v2-abilities-first-6e12027c36c9)

## Context

[ADR-0027](0027-mcp-dual-mode.md) established DailyOS as both an MCP server (exposing workspace tools and resources to external agents) and an MCP client (consuming external services). The v1.5.0 project description is sharp on guarantees: tools must route through abilities, not parallel paths; writes must capture structured intent and provenance; reads must return current substrate state; claim-worthy outputs must explain source, freshness, confidence, and salience.

What is missing is positioning. The existing scope tells contributors *what guarantees not to break*. It does not tell them *what the product is from a user's seat*.

This ADR fills that gap. It does not change the v1.5.0 substrate scope. It frames it.

The frame is straightforward: **MCP is not an export pipe; it is a co-equal head over the same intelligence substrate as the Tauri app.** The Tauri app and the MCP head are two different consumption optimizations of one claim graph.

- The **Tauri head is opinionated**: briefing → triage → prep, with hierarchy and callouts the user reads visually. Designed for the daily ritual.
- The **MCP head is responsive**: whatever question came in, answer it well inside the host model's conversation. Designed for the on-demand ask.

Same substrate, different consumption shapes for different moments.

This framing has product consequences. Treating MCP as plumbing produces a generic data API. Treating MCP as product produces a tool surface as carefully designed as any UI in the app — and a tool description corpus that is itself the highest-leverage user-facing copy in the product, because every host-model conversation routes through it.

## Decision

### 1. Headless DailyOS as the canonical frame

The MCP server is the **headless head** of DailyOS. It exposes the same intelligence substrate as the Tauri app, optimized for a different consumption surface (host-model conversation rather than direct visual reading).

Architectural implication: the substrate is the product; the heads are surfaces over it. Adding a new surface (CLI, mobile, browser extension, future agent shell) follows the same pattern. This generalizes [ADR-0111](0111-surface-independent-ability-invocation.md): a Surface invokes abilities; a Surface is not a feature implementation.

The heads are co-equal *as consumption optimizations*, not feature-parity-mandated. The MCP head does not need to expose every Tauri view, and the Tauri app does not need to mimic conversational interaction. Each surface picks the questions and ritual it serves best.

### 2. Tools are question-shaped, not data-shaped

MCP tool design is governed by the questions a user would ask through their host model, not by the rows in the database.

- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
- Rejected: `get_account(id)`, `list_claims(filter)`, `search(q)` — generic data accessors that bypass synthesis and force the host model to reassemble context.

A tool that does not exercise an ability is a candidate for removal. This extends the v1.5.0 project Scope Guidance: convenience tools that do not strengthen parity with the DailyOS app loop are out of scope.

### 3. Tool descriptions are UI copy

Tool names and descriptions are the highest-leverage user-facing copy in the product. They determine when a host model reaches for DailyOS over alternatives (Glean, Slack search, web search), how it cites, and how it hedges.

Each tool description should:

- State the question shape it serves.
- State when to prefer it over likely alternatives the host model has access to.
- State the freshness model — DailyOS persists context across conversations and updates as work evolves.
- Match the voice and vocabulary of [ADR-0083](0083-product-vocabulary.md) — same product diction as the Tauri app.

Tool description copy is reviewed with the same care as a button label or section header.

### 4. Claim-shaped output, with provenance leaking through

Tool outputs are claim-shaped, not document-shaped. Each user-meaningful field carries inline trust band and source attribution per [ADR-0105](0105-provenance-as-first-class-output.md). The output structure should make it natural for a host model to render hedging when trust is low — *"according to a transcript from last Thursday, though this hasn't been confirmed elsewhere…"* — without explicit prompting.

The substrate leaking through into the conversation *is* the product differentiator. It is what makes a DailyOS-mediated answer feel different from a flat retrieval over a corpus.

### 5. Feedback is the only write — same posture, different surface

Consumption-first holds in the headless head. The MCP write surface is **feedback only**: corrections, dismissals, corroborations, contradictions, and tombstones, all flowing through the existing claim feedback path defined in [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).

The MCP head does not expose:

- Claim creation or direct edit.
- File generation, drafting, or sending.
- Calendar mutation, message composition, or external-system writes.

A user in Claude Desktop can correct what DailyOS knows. They cannot use DailyOS as a write layer. Drafting and sending happen in the tool that's good at writing, with DailyOS context piped in.

This preserves the v1.5.0 user outcome ("writes capture structured intent and provenance") in spirit by routing all writes through the same propose/commit boundary the Tauri app uses, with the same actor distinctions per [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).

### 6. Cross-conversation continuity is a named affordance

Host-model conversations are stateless. DailyOS is not. A user asking the same question across two Claude Desktop conversations hits the same evolving substrate.

This is a quiet superpower and should be made explicit in the surface itself, not left as an implementation detail. Tool descriptions name it. Returned objects, where appropriate, surface it ("last verified Tuesday; no contradicting signals since"). The host model is given the language to communicate substrate persistence to the user.

### 7. CLI as a third head, mediated by MCP

The same headless framing extends to a CLI surface. CLI is a third head over the substrate, not a separate substrate consumer.

**Transport.** CLI invocations route through the MCP server. There is no parallel CLI ingress to the abilities runtime. The default is MCP-mediated; a CLI-distinct transport is only justified if something genuinely cannot be expressed through the MCP surface, and that decision is deferred until the need is concrete. Keeping a single ingress to the abilities runtime is easier to reason about, easier to evolve, and prevents the v1.5.0 mission gate ("don't bypass guarantees") from being silently undermined by a second path.

**Render.** CLI's distinct contribution is the output shape. Three renders over one substrate:

- Tauri: substrate → React components
- MCP: substrate → claim-shaped JSON
- CLI: substrate → markdown

Markdown is the right CLI render because it is both terminal-readable and vault-compatible. A `dailyos brief` command emits markdown a user can read in the terminal, pipe to a file, or paste into Obsidian. This actively reinforces the "you always keep your files" defensibility position established by the v0.0.1 PARA + markdown foundation — the substrate's intelligence is reachable as markdown, not only as a GUI.

**Two consumption modes share this transport:**

- **CLI-for-agent.** A Claude Code skill, or any agent that shells out, invokes the CLI as part of a workflow. Skills are *macros over the MCP surface* — named, slash-invokable bundles of MCP calls with workflow shape. The agent is the consumer; the user is one step removed. Structurally similar to MCP-via-host-model.
- **CLI-for-human.** A developer or power user invokes the CLI directly in a terminal. Direct consumption like the Tauri app, text-shaped instead of visual-shaped.

**Scope discipline is unchanged.** CLI commands mirror the **app loop** (briefing, prep, account status, commitments, action review), not the **org corpus** (Glean's territory). The displacement framing in §The displacement use case applies regardless of transport.

**Existing skills bundles.** Distributable Claude Code skills already exist in the repository:

- `plugins/dailyos/skills/` — workspace-fluency, entity-intelligence, meeting-intelligence, political-intelligence, action-awareness, relationship-context, analytical-frameworks, role-vocabulary, loop-back.
- `plugins/dailyos-writer/skills/` — writer-core plus voice/structural/authenticity/mechanical review.

These predate the abilities-first MCP surface. They should be refreshed once v1.5.0 lands so they consume the new tool surface and inherit its descriptions, output shape, and feedback semantics. This is a v1.5.x follow-on, not v1.5.0 substrate work — but it is the natural beneficiary of v1.5.0 once the MCP head is solid.

## The displacement use case

This ADR adopts a worked example as a design check for v1.5.0 issues: the **Glean displacement** scenario.

A user is in Claude Desktop with both a Glean MCP and a DailyOS MCP connected. They ask: *"What's going on with Acme Corp?"*

The desired behavior: the host model reaches for DailyOS, not Glean.

Why DailyOS wins this question:

| Dimension | Glean | DailyOS |
|---|---|---|
| Corpus | Org-wide documents and messages | The user's working substrate |
| Output | Retrieval (matched documents) | Synthesis (current claim state) |
| Freshness | Updates when content is written | Updates when *anything* happens, with re-evaluation |
| Trust | Flat | Trust bands per claim |
| Permissions | Org policy | The user's local-first DB |

DailyOS does not replace Glean. Glean remains the right substrate for org-wide policy, how-to, and broad-corpus search. The two are complements with a clean seam:

> **DailyOS for the user's working understanding of their professional world; Glean for the corpus of the company they work at.**

The MCP surface succeeds at this question when the host model reaches for DailyOS *because the tool descriptions taught it to*, not because we blocked Glean. Each v1.5.0 tool description should be reviewable against this scenario.

The same complement-not-replacement framing extends to other corpus tools the host model may have connected: Notion search, Slack search, web search. The seam is always the same — DailyOS for *the user's working understanding*, the other tool for *its corpus*.

## Non-goals

Restated and extended from the v1.5.0 project description:

- **Not a wrapper for Claude.** DailyOS does not host conversation. The user converses in their host model of choice; DailyOS is the substrate the host model consults.
- **Not a write layer.** Drafting, generation, sending, and external-system mutation are out of scope. Feedback corrections are not "writes" in the user-facing sense; they are the same propose/commit primitive the Tauri app uses.
- **Not a Glean replacement.** Org-wide retrieval is not what DailyOS does. Personal-context synthesis is.
- **Not a parallel feature surface.** (Already established in v1.5.0 project description; restated here as it is a corollary of §1.)
- **Not a generic data API.** Tools that do not exercise abilities are not added for completeness.

## Consequences

### Positive

- v1.5.0 issue scoping has a positioning lens. *"Does this tool serve a question a user would ask?"* and *"Would this tool description steer a host model toward DailyOS over Glean for the right questions?"* become reviewable design questions.
- Tool descriptions become a recognized work product, not an afterthought. Likely produces a tool surface inventory deliverable under v1.5.0 (tool name, description, expected question shape, return shape, displacement test).
- The headless-head metaphor generalizes. Future surfaces (CLI, mobile, browser extension, agent shells) inherit the same posture without re-litigation.
- The displacement use case gives partnership and positioning conversations a concrete frame. DailyOS and Glean are complements, not competitors. Same posture extends to other corpus tools.
- Reinforces the consumption-first product principle established implicitly across v1.4.x by making it explicit and surface-portable.

### Negative / risks

- "Headless" is metaphorical. Some contributors may read it as "MCP must implement every Tauri surface," which is not the intent. §1 must be explicit about co-equal-as-consumption-optimization, not feature-parity.
- Tool description copy is high-leverage and low-iteration — it ships once and steers thousands of host-model conversations. Recommends a review process analogous to product copy review.
- The displacement framing assumes a host model with multiple MCPs connected. For users with only DailyOS connected, the comparison is moot but the descriptions still teach the host model how to reason about coverage.
- Feedback-only writes constrain integration depth. A user who wants Claude Desktop to "create a Linear ticket" via DailyOS will be redirected — that mutation goes through Linear's MCP or Claude Desktop's own tools, not ours. We accept this; the alternative is becoming a write layer, which the v1.5.0 project explicitly is not.

### Neutral

- This ADR adds no runtime code. Framing only.
- It does not change the v1.5.0 substrate scope. It is consumed by v1.5.0 issue review and tool surface design.
- A separate ADR may be warranted later to cover ingestion-side architecture (unifying inbox processing, pollers, and external sources under a common event abstraction). That work is out of scope here and unrelated to the consumption surface this ADR governs.

## References

Internal:

- [ADR-0027](0027-mcp-dual-mode.md) — MCP integration: dual-mode server + client (foundation)
- [ADR-0083](0083-product-vocabulary.md) — Product vocabulary discipline
- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities as runtime contract
- [ADR-0105](0105-provenance-as-first-class-output.md) — Provenance as first-class output
- [ADR-0111](0111-surface-independent-ability-invocation.md) — Surface-independent ability invocation
- [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) — Human and agent analysis as first-class claim sources (feedback semantics)
- [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) — DailyOS as an AI harness (principles cross-reference)
- [v1.5.0 — MCP Server v2 (Abilities-First)](https://linear.app/a8c/project/v150-mcp-server-v2-abilities-first-6e12027c36c9)

External:

- [Anthropic — Model Context Protocol](https://modelcontextprotocol.io/) — tool description as model-facing affordance
