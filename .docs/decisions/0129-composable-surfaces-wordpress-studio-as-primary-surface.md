# ADR-0129 — Composable Surfaces: DailyOS as Runtime, WordPress Studio as Primary Surface

**Status:** Proposed
**Date:** 2026-05-10
**Amended:** 2026-05-10 — §4 reframed (WP path uses WP Abilities API + MCP Adapter, not PHP MCP client) per Codex L0 Prep follow-up on DOS-546. Primary-source research confirmed Abilities API in WordPress core since 6.9 (December 2025); MCP Adapter ships at `github.com/WordPress/mcp-adapter`.
**Authors:** James Giroux, Claude
**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md), [ADR-0127](0127-presets-as-intelligence-contracts.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md)
**Linear:** [v1.4.1 — Abilities Runtime Completion](https://linear.app/a8c/project/v141-abilities-runtime-completion-19eb22af50e7), [v1.4.2 — Entity Intelligence](https://linear.app/a8c/project/v142-entity-intelligence-accounts-projects-people-a448d2e072ad), [v1.4.7 — MCP Server v2](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9), [v1.4.9 — Self-Healing v2](https://linear.app/a8c/project/v149-self-healing-v2-da4bc5769164), [DOS-540](https://linear.app/a8c/issue/DOS-540)

## Context

[ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) established the architectural frame: *the substrate is the product; the heads are surfaces over it.* It named two heads — the Tauri app (visual, opinionated, daily ritual) and the MCP server (responsive, headless, on-demand) — and explicitly anticipated more: *"Adding a new surface (CLI, mobile, browser extension, future agent shell) follows the same pattern."*

This ADR commits to that surface-pluggable architecture concretely, and at a moment that matters for the cost of doing so.

### What's true today

- v1.4.0 (Abilities Runtime Spine) shipped. v1.4.1 (Abilities Runtime Completion) is in flight and finishing the substrate.
- v1.4.2 onward (Entity Intelligence, Briefing Experience, Claim/Trust UI, Salience, Workspace Memory, MCP Server v2, Reports, Self-Healing) is about to begin significant interface design and implementation work — the "redesign tokens" of the program.
- The current primary surface is a Tauri (Rust + React) macOS app rendering a magazine reading surface. The substrate work in v1.4.0/v1.4.1 is largely surface-agnostic; the substantial surface investment is still ahead, not behind.
- The MCP head exists in v1.4.7's Linear scope as a late-program deliverable.

### What's shifted in the field

- **MCP is now genuinely cross-vendor.** Spec 2025-11-25 with OAuth 2.1, Step-Up Auth, Async Tasks. OpenAI and Google ship MCP clients in production; the standard is no longer Anthropic-led.
- **Frontier model rendering is moving from markdown to HTML.** Anthropic engineers and adjacent practitioners are advocating HTML over markdown for AI-rendered output: richer semantic structure, inline interactivity, embedded media, better composability with the web ecosystem, better meaning-to-character ratio at high complexity. The "markdown second brain" posture is increasingly a starting-point ergonomic, not a target architecture.
- **Local CMS is the natural endpoint of the file-system-as-second-brain trajectory.** Markdown vaults (Obsidian, Logseq), wiki-page systems (GBrain, Notion), and block-composable tools converge on the same shape: typed pages with structured fields, composable blocks, plugin-extensible capability, and a render layer richer than flat text. WordPress has been this shape for editorial publishing since 2018 (Gutenberg) and is now mature enough to serve non-publishing knowledge surfaces.
- **WordPress Studio** ships local WordPress as a desktop application. Local-first, fast install, no server requirement, with one-click sync to WordPress.com when the user chooses to publish. It is, in effect, "local CMS as a desktop app" — a category that did not exist as a credible product line two years ago.
- **WordPress Remote Data Blocks** allow Gutenberg blocks to compose external data with authentication and server-side rendering. The pattern of *"a block that pulls live data from an external source and renders inline"* is shipped infrastructure, not a feature we'd build.
- **Embedded agent SDKs and managed agent harnesses** (Claude Agent SDK, Anthropic Managed Agents beta, OpenAI Agents SDK, Gemini Enterprise Agent Platform) make BYOM-shaped runtimes viable. ACP (Zed's Agent Client Protocol) is the leading candidate for a vendor-neutral host-to-agent seam.

### Why now

The economic argument is the load-bearing one. v1.4.2 onward will spend significant design and engineering effort on entity surfaces, briefing experience, claim/trust UI, salience surfaces, and reports rendering. **Those redesign tokens are spent once.** Spending them on a Tauri React magazine surface and re-spending them on a WordPress block surface six to twelve months later is paying twice. Spending them on the WordPress block surface from the start is paying once.

The cheapest moment to commit a surface pivot is the moment before the new-surface design work begins in earnest. That moment is now — between v1.4.1 finishing and v1.4.2 starting. Six months later, the redesign tokens are sunk into Tauri React surfaces and the pivot costs materially more.

### Why this ADR (vs. ADR-0128's existing framing)

ADR-0128 framed the architecture as surface-pluggable. It treated the Tauri app as the primary visual surface and MCP as a co-equal headless head. This ADR commits to the next step in that architecture: **WordPress Studio is added as a new head and is promoted to primary**, with the Tauri app reorienting toward runtime-host duties and the MCP head elevating from a late-program feature to the foundation contract every v1.4.2+ wave consumes.

This ADR does not contradict ADR-0128. It is the operationalization of ADR-0128's architectural thesis at the moment the substrate has matured enough to make pluggable surfaces real.

## Decision

### 1. DailyOS is a runtime; surfaces are clients

The product is the personal-intelligence runtime: substrate (claims, trust, lifecycle, signals, provenance), abilities runtime, agent-backend mediation, local-first data sovereignty. Surfaces are clients of the runtime. No surface is the product; no surface is sacred.

This generalizes ADR-0128 §1 from *"the substrate is the product; the heads are surfaces over it"* to: *"the runtime is the product. The substrate is its memory; abilities are its capability surface; surfaces are how users compose against it."*

Brand and positioning reframe accordingly: from *"DailyOS is a Mac app for personal intelligence"* to *"DailyOS is your personal intelligence runtime; it works wherever you compose."*

### 2. WordPress Studio is the primary composable surface

The primary visual surface where users compose, read, edit, and share their intelligence becomes a local WordPress install — bundled via WordPress Studio for new users, or any local WordPress installation for users who already run one.

DailyOS ships:
- A custom block library that renders substrate types (entity blocks, claim blocks with trust-band rendering, briefing blocks, callout blocks, prep blocks, report blocks, agentic blocks).
- A magazine theme implementing the editorial reading surface, finite endings, hierarchy, and product vocabulary discipline (per [ADR-0083](0083-product-vocabulary.md)).
- Custom post types for the entity model (Account, Project, Person, Meeting, Briefing, Report) as projections over the substrate.
- A plugin that mediates the substrate-to-WordPress integration: read APIs, write APIs (corrections / dismissals / corroborations), event subscriptions, identity bridge.
- Composition templates for daily briefings, account pages, meeting prep, and report drafts — anchored shapes the user can deviate from. Layouts may also be **agent-composed on the fly** based on substrate salience, not bound to fixed templates.

Local-first holds. Privacy holds. Open-format holds. The substrate remains in user files; WordPress is the composition layer, not the storage of record.

### 3. The substrate-to-surface contract is foundation work, not a late-program feature

The contract between the substrate and any surface — typed read API, typed write API, event subscription, composition primitives, identity and scope — is the load-bearing layer for every v1.4.2+ wave. It cannot be a late-program feature.

This ADR reorders the program: the substrate-to-surface contract (currently scoped inside v1.4.7) is **promoted to a foundation wave between v1.4.1 and v1.4.2**. v1.4.7's MCP server work continues as the first external implementation of the contract; the contract itself ships earlier so that v1.4.2 entity surfaces, v1.4.3 briefing surfaces, v1.4.4 claim/trust surfaces, and v1.4.5 salience surfaces all consume the same substrate-to-surface API.

### 4. WordPress accesses the substrate via its own Abilities API + MCP Adapter; non-WP surfaces use the Rust runtime's MCP server directly

*Reframed 2026-05-10 per Codex L0 Prep follow-up on DOS-546. The original framing — "WordPress plugin opens an MCP session to the localhost-bound runtime" — was based on a PHP MCP client architecture that primary-source research showed isn't necessary and isn't the canonical WP integration target.*

The runtime exposes substrate-backed abilities through two paths, both consuming the same substrate:

- **WordPress path.** The DailyOS WP plugin registers abilities into [WordPress's Abilities API](https://developer.wordpress.org/news/2025/11/introducing-the-wordpress-abilities-api/) (in WordPress core since 6.9, December 2025; client-side API merged into Gutenberg for 7.0, shipping 2026-05-20). Those abilities delegate to the Rust runtime over a local transport (loopback HTTP REST as primary candidate; Unix domain socket as fallback; spawned stdio as last-resort). The [WordPress MCP Adapter plugin](https://github.com/WordPress/mcp-adapter) exposes registered abilities as MCP tools automatically — WordPress itself becomes an MCP server consumable by Claude Desktop, Cursor, VS Code, and any other MCP client connected to the WP site. Gutenberg blocks consume the abilities through WP-native infrastructure (REST endpoints, capabilities, nonces, and the WP 7.0 client-side `executeAbility()`).
- **Non-WP path.** The Rust runtime exposes its own MCP server for surfaces that don't sit inside WordPress (Claude Desktop direct without a WP install, Cursor, future PWA, future mobile). Same substrate, two MCP servers — one WP-mediated for WP-consuming clients, one direct for everything else.

DailyOS does **not** consume MCP from PHP. The seam between WordPress and the runtime is a local PHP-to-Rust transport, not an MCP client; the WP Abilities API + MCP Adapter does the MCP server work on the WP side.

This preserves [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §7's discipline (single ingress to the abilities runtime) — every external invocation, whether routed through the WP MCP server or the runtime's direct MCP server, ultimately resolves to one substrate access path inside the runtime. The runtime exposes one substrate access protocol; clients vary in how they render and compose against it.

**MCP Adapter exposure policy.** Substrate-backed abilities must NOT be exposed by the *default* WP MCP server. DailyOS ships a custom MCP server configuration with an explicit ability allowlist, a dedicated low-capability WP user for substrate access, and read-mostly defaults. Permission callbacks check both WP capabilities and DailyOS `SurfaceClient(WordPress)` scopes (per [ADR-0111](0111-surface-independent-ability-invocation.md)); site-wide WP capabilities alone are not the isolation boundary.

### 5. Free vs. paid as architectural and business shape

The free / paid line maps cleanly to the runtime vs. shared-infrastructure distinction, mirroring WordPress's own gravity (WordPress.org self-hosted vs. WordPress.com / VIP hosted).

- **Free.** Local WordPress (Studio or any local install) + DailyOS plugin/theme/block bundle + local Rust runtime + BYOM agent backend. Single-user. Sovereign. No DailyOS-hosted infrastructure required for it to work end-to-end.
- **Paid.** Hosted multi-tenant substrate. Shared surfaces across teams. Enterprise performance characteristics (hosted vector DB, knowledge-graph-shaped reads). Optional DailyOS-hosted agent backend for users who don't want to manage one. Multisite-shaped team boundaries. The WordPress VIP playbook applied to personal-becoming-team intelligence.

The free tier is genuinely free in capability, not freemium-crippled. The paid tier earns its money on shared infrastructure, multi-user coordination, and enterprise-grade hosting — work the free tier doesn't need.

### 6. Block-shaped composition; agentic blocks as a first-class primitive

Gutenberg blocks are typed HTML structures with editable attributes. They are the right shape for AI-generated, user-editable, persistently-composed intelligence — the bridge between *"AI generates structured output"* and *"user composes / edits / refines."*

Three composition modes coexist:

- **Templated layouts.** Curated page templates (daily briefing, account page, meeting prep, report) that the user can deviate from. Anchored shapes.
- **On-the-fly composition.** The agent composes block sets at runtime based on substrate salience. Today's account page may differ from yesterday's because what's salient changed. The user can pin templates as anchors but the system isn't bound to them.
- **Agentic blocks.** A "DailyOS Ask" block, a "prep this meeting" block, a "summarize this" block, a "forecast this account" block. Embedded inline in any page, in context with the user's other blocks, editable, savable, persistent. *Not a chat sidebar — composable agentic primitives the user assembles into their own pages.*

This is the rendering posture the HTML-over-markdown trajectory points toward, and it composes naturally with the typed substrate the v1.4.x program has been building.

### 7. The Tauri app reorients toward runtime-host duties

The Tauri app continues to exist. Its role reorients:

- **Runtime host.** Rust services, cron, the MCP server, local privacy gate, abilities execution, signal propagation, claim writes. This is what Tauri does well and continues to do.
- **UI role transitions.** The current React magazine surface is no longer the primary visual surface. Whether it (a) deprecates, (b) becomes a power-user / developer surface complementary to WordPress, or (c) becomes a thin admin/status surface for runtime visibility — is deferred to empirical evaluation after the WordPress surface stabilizes. This ADR does not lock in that decision.

The Rust substrate engine, abilities runtime, signal propagation, claim feedback path, and MCP server all continue exactly as v1.4.0/v1.4.1 designed them. None of that work is wasted. The investment has always been in the runtime; this ADR makes that explicit.

### 8. ACP-shape as the pluggable agent backend seam

The host-to-agent backend interface is shaped by the principle that DailyOS is a bring-your-own-model platform. The user picks Claude Agent CLI, Codex, Gemini CLI, or any future ACP-compatible agent, and DailyOS speaks to it through a session-shaped protocol.

ACP (Zed's Agent Client Protocol) is the leading candidate for the protocol implementation. The architectural commitment is to the **shape** (out-of-process agent, session-shaped invocation, MCP-shaped tool surface), not to the specific protocol. Implement ACP first as the leading candidate; leave room for a second protocol if a different one matures faster.

This composes with §4: agents reach the substrate via MCP; surfaces reach the substrate via MCP; agent backend is pluggable; surface is pluggable; runtime is the constant.

### 9. Wave-program reorientation

v1.4.x wave program after this ADR:

- **v1.4.1** — Abilities Runtime Completion. Finishes as planned. Surface-agnostic substrate work.
- **(New foundation wave between v1.4.1 and v1.4.2)** — substrate-to-surface contract. Formerly part of v1.4.7's scope. Typed read/write/subscribe APIs, composition primitives, identity bridge. MCP server is the first external implementation.
- **v1.4.2** — Entity Intelligence, WP-shaped. Custom post types, entity blocks, theme treatments.
- **v1.4.3** — Briefing Experience, WP-shaped. Composable briefing as a Gutenberg page template / on-the-fly composition.
- **v1.4.4** — Claim Experience & Trust UI, WP-shaped. Block-level claim primitives with trust-band rendering; corrections feed substrate via the contract.
- **v1.4.5** — Salience & Recommendations, WP-shaped. Drives block visibility and ordering.
- **v1.4.6** — Workspace Memory Refactor. Substrate work, minimal pivot impact. Markdown filesystem ↔ WordPress DB reconciliation lives here.
- **v1.4.7** — MCP Server v2. Repurposed: the contract foundation has shipped earlier; v1.4.7 now focuses on the rich tool surface, displacement-test discipline (per ADR-0128), and the cross-surface story.
- **v1.4.8** — Reports as Shareable Intelligence. Maps directly: reports become curated WordPress posts/pages with WP.com sync as the publish target.
- **v1.4.9** — Self-Healing v2 + Skillify ([DOS-540](https://linear.app/a8c/issue/DOS-540)). Abilities-as-plugins fits naturally in WordPress's plugin model.

Specific wave scope adjustments are tracked in their respective Linear projects after this ADR is accepted. This ADR sets direction; the project descriptions absorb the implications.

### 10. Validation before full commit

The pivot is the call. The validation is the discipline.

A bounded prototype spike — 2 to 4 weeks — validates two load-bearing assumptions before v1.4.2 begins WordPress-shaped work in earnest:

- **Instant-launch feel.** A WordPress Studio briefing render with realistic claim volume — does it hit the *"open the app, your day is ready"* feel? Cold-load < 2s, warm-load < 500ms, with appropriate caching and pre-rendering. Empirical answer beats armchair.
- **Markdown filesystem ↔ WordPress DB write path.** Gutenberg block save → substrate event → markdown projection. Does the three-view consistency model (substrate of record, WP DB as projection, filesystem as durable archive) hold up under realistic editing patterns?

Exit criteria: commit pivot, hold for one more iteration of design work, or fall back to extended Tauri React surfaces. This is the only place this ADR creates an explicit gate.

## Non-goals

- **Not abandoning the Tauri runtime.** The Rust substrate, abilities runtime, signal propagation, MCP server, cron, and local privacy gate continue exactly as v1.4.0/v1.4.1 design. The runtime is the product.
- **Not requiring WordPress to consume DailyOS.** Headless surfaces (Claude Desktop via MCP, Cursor, future) work without WordPress. WordPress is the *primary* composable surface, not the *only* surface.
- **Not a WordPress plugin / theme product.** The product is the runtime. The WordPress integration is one surface implementation, distributed as a plugin/theme bundle, but the product identity is the runtime, not the WP bundle.
- **Not committing to multi-tenant hosted substrate as v1.4.x scope.** Paid-tier hosting is a future architectural commitment, not v1.4.x work. The free-tier capability proves the substrate; the paid-tier engineering follows once free-tier customers exist.
- **Not changing local-first, privacy, BYOM, or the substrate's typed model.** Trust as substrate property, claims with provenance and lifecycle, personal-not-organizational — all preserved.
- **Not deprecating ADR-0128.** This ADR extends ADR-0128's surface-pluggable thesis with a concrete primary-surface choice. The headless / co-equal-heads frame from ADR-0128 stays canonical.

## Consequences

### Positive

- **Redesign tokens are spent on the right surface.** The substantial design and engineering effort in v1.4.2-v1.4.9 lands directly on the surface that ships, not on a Tauri React surface that gets reshaped later.
- **Plugin ecosystem solves the dynamic-ability question structurally.** Skillify-extracted abilities ([DOS-540](https://linear.app/a8c/issue/DOS-540)) become WordPress plugins. The "is the registry sealed?" architectural concern dissolves into WordPress's plugin loader.
- **Block composability is the right rendering substrate for AI-generated, user-editable intelligence.** Gutenberg blocks bridge generation and editing in a way React components do not.
- **Significant infrastructure leverage from WordPress.** REST and GraphQL APIs satisfy the substrate-to-surface contract on the read side without bespoke transport. Remote Data Blocks ship the "block reads from external source with auth" pattern. Multisite gives team boundaries for paid tier. WP.com sync gives shareable-output publishing. VIP infrastructure gives the enterprise-hosting playbook.
- **Cross-surface story becomes coherent.** WordPress for primary composition; Claude Desktop and future agent shells for headless conversation; substrate sovereign. Each surface picks the questions and ritual it serves best, per ADR-0128 §1.
- **Brand reframe ("runtime, not Mac app") is sharper and more durable.** Decouples the product identity from any one surface. New surfaces (mobile, voice, future-thing) become additive, not replacement.
- **Free vs. paid line is structurally clean.** Mirrors WordPress's own model. Users opt into the mental model already.

### Negative / risks

- **Brand reframe is real.** "DailyOS is a Mac app" → "DailyOS is your personal intelligence runtime" is a positioning shift that needs comms work, not just architecture work.
- **First-run experience changes.** "Install Studio, then install DailyOS bundle" is a different onboarding from "download a Mac app." Mitigations exist (Studio profile / one-click bundle install) but the change is real.
- **Markdown ↔ WordPress write path needs design.** Substrate of record, WP DB as projection, filesystem as durable archive. Three-view consistency is solvable but requires care. Validated in the prototype spike per §10.
- **Existing Tauri React design effort is redirected, not lost.** Design tokens (CSS variables) port directly; visual primitives (typography, spacing, color, hierarchy) survive a rendering-stack swap; component-level work (React components → Gutenberg blocks) is a translation, not a rewrite. But the translation is real work.
- **Native Mac app users (small cohort, including James as customer-zero) experience a transition.** Section 7 defers the Tauri UI's specific fate; users see a transition either way. Communication and pacing matter.
- **WordPress aesthetic baggage is a real perception risk.** Mitigated by a custom magazine theme committed to the design system, but the risk exists in marketing and first impressions.
- **WordPress Studio is younger than mature OS apps.** Cross-platform parity (especially Windows) and stability under continuous use are still maturing. Studio's trajectory is favorable but the dependency is real.
- **MCP and ACP maturity dependencies.** MCP is mature and cross-vendor; ACP is younger. Section 8 mitigates by committing to the *shape* not the protocol.

### Neutral

- **No substrate code changes.** The runtime continues as designed. Surface work is additive (new WP integration plugin/theme) and reorientative (v1.4.2+ targets a different render stack).
- **ADR-0128 is preserved and extended.** The surface-pluggable thesis stays canonical; this ADR commits to the next step.
- **The wave program structure stays similar.** Wave count, scope shape, and substrate trajectory are mostly unchanged. Implementation tech changes for surface-touching waves.
- **The prototype spike is the only explicit gate.** This ADR commits to direction; the spike is the empirical check that the load-bearing assumptions hold.

## References

Internal:

- [ADR-0027](0027-mcp-dual-mode.md) — MCP integration: dual-mode server + client (foundation)
- [ADR-0083](0083-product-vocabulary.md) — Product vocabulary discipline
- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities as runtime contract
- [ADR-0105](0105-provenance-as-first-class-output.md) — Provenance as first-class output
- [ADR-0111](0111-surface-independent-ability-invocation.md) — Surface-independent ability invocation
- [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) — DailyOS as an AI harness
- [ADR-0127](0127-presets-as-intelligence-contracts.md) — Presets as intelligence contracts
- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS: MCP as a co-equal product surface (this ADR extends)
- [DOS-540](https://linear.app/a8c/issue/DOS-540) — Skillify proposal (capability-gap wave)
- [v1.4.1 Linear project](https://linear.app/a8c/project/v141-abilities-runtime-completion-19eb22af50e7)
- [v1.4.2 Linear project](https://linear.app/a8c/project/v142-entity-intelligence-accounts-projects-people-a448d2e072ad)
- [v1.4.7 Linear project](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9)
- [v1.4.9 Linear project](https://linear.app/a8c/project/v149-self-healing-v2-da4bc5769164)

External:

- [WordPress Studio](https://developer.wordpress.com/studio/) — local WordPress as a desktop app
- [WordPress Remote Data Blocks](https://github.com/Automattic/remote-data-blocks) — Gutenberg blocks consuming external authenticated data
- [Model Context Protocol Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
- [Zed Agent Client Protocol (ACP)](https://zed.dev/blog/agent-client-protocol) — vendor-neutral host-to-agent seam
- [Anthropic — Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)
- [Anthropic — Harness design for long-running application development](https://www.anthropic.com/engineering/harness-design-long-running-apps)
