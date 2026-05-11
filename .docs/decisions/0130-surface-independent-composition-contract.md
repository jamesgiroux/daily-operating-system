# ADR-0130 — Surface-Independent Composition Contract

**Status:** Proposed
**Date:** 2026-05-10
**Amended:** 2026-05-10 — added explicit references to existing substrate retrieval primitives (ADR-0074 vector search, ADR-0078 embed model) in §2 (Salience computation) and §3 (Custom block fallback) to prevent composition-producing abilities from reinventing retrieval.
**Authors:** James Giroux, Claude
**Relates to:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md), [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md)
**Linear:** [DOS-546](https://linear.app/a8c/issue/DOS-546) — WordPress Studio surface viability spike (validates this contract for the WP instance)

## Context

DailyOS has converged on a runtime + composable surfaces architecture (ADR-0128 surface-pluggable thesis; ADR-0129 WordPress as a leading concrete surface; ADR-0111 §8 generalized SurfaceClient as the fourth actor class for third-party local surfaces). The substrate-side contracts are in place:

- **Abilities** ([ADR-0102](0102-abilities-as-runtime-contract.md)) — typed runtime capabilities.
- **Provenance** ([ADR-0105](0105-provenance-as-first-class-output.md)) — claim-shaped output with source attribution and trust bands.
- **Invocation** ([ADR-0111](0111-surface-independent-ability-invocation.md)) — bridge-per-surface ability invocation with actor-filtered discovery.
- **Rendering** ([ADR-0108](0108-provenance-rendering-and-privacy.md)) — provenance rendering per surface/actor.

What's missing is the contract for *composition*: how a substrate-produced briefing, account page, prep pack, or report is described independently of any rendering technology. ADR-0129 §6 commits to Gutenberg blocks as "the right shape" for composed surfaces, but does not separate the *generic* shape (typed blocks composed into structured pages) from the *implementation* (Gutenberg). Without that separation, WordPress quietly becomes more than a surface — it becomes the document model. That cuts against ADR-0129 §1's own claim that no surface is sacred.

The L0 Prep follow-up on DOS-546 (Codex Pass 2 formal L0, 2026-05-10) named this gap explicitly: "The biggest decision L0 should force is canonical composition authority. ADR-0129 commits to Gutenberg blocks as 'the right shape', but does not decide whether DailyOS has its own surface-independent composition IR. It should." This ADR fills that gap.

## Decision

### 1. The substrate owns composition

Composed surfaces (briefings, entity pages, prep packs, reports, callouts, agentic-block compositions) are described by substrate primitives independent of any rendering technology. Surfaces render the composition; surfaces do not author it.

This generalizes [ADR-0111](0111-surface-independent-ability-invocation.md) §1 from "surfaces invoke abilities" to "surfaces consume substrate output, including composed pages." It generalizes [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §1 from "the substrate is the product; the heads are surfaces over it" to apply at the *page* level, not just the *claim* level.

### 2. Composition primitives

The substrate emits composed output as instances of a typed `Composition` model. The model:

```rust
pub struct Composition {
    pub id: CompositionId,
    pub kind: CompositionKind,  // Briefing | EntityPage | PrepPack | Report | Callout | Custom
    pub subject: Option<EntityRef>,  // The entity this composition is about, if any
    pub sections: Vec<Section>,
    pub salience: Salience,         // Top-level relevance weight
    pub provenance: ProvenanceEnvelope,  // Per ADR-0105
    pub generated_at: DateTime<Utc>,
    pub generated_by: AbilityRef,  // Which ability produced this composition
}

pub struct Section {
    pub id: SectionId,
    pub label: Option<String>,    // Optional editorial heading
    pub blocks: Vec<Block>,
    pub salience: Salience,
}

pub struct Block {
    pub id: BlockId,
    pub block_type: BlockType,    // Typed taxonomy; see §3
    pub payload: BlockPayload,    // Schema varies by block_type
    pub claim_refs: Vec<ClaimRef>, // The claims this block renders
    pub provenance: ProvenanceEnvelope,
    pub salience: Salience,
    pub render_hints: RenderHints,  // Surface-neutral hints (emphasis, density, etc.)
}

pub struct Salience {
    pub weight: f32,           // 0.0 - 1.0
    pub band: SalienceBand,    // critical | important | contextual | background
    pub reason: String,        // Why this is at this weight
}

pub enum SalienceBand { Critical, Important, Contextual, Background }
```

Provenance and trust bands flow through the composition at the block level, not as a footer. A renderer that strips provenance is rendering wrong; the contract treats provenance as part of the composition's meaning, not its decoration.

**Salience computation uses existing substrate primitives.** The `Salience.weight` field is typically computed via the substrate's hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)) over the embed model ([ADR-0078](0078-nomic-embed-text-model-switch.md)), combined with signal correlation, recency, and claim-trust factors. The Composition contract does not mandate the scoring algorithm — it names the substrate's existing retrieval primitives as the canonical inputs. **Composition-producing abilities MUST NOT reinvent retrieval.** They consume the substrate's embed-model-backed search; the substrate owns the relevance question. This rule prevents the failure mode where an ability author rebuilds salience scoring from scratch without realizing the substrate already has it.

### 3. BlockType taxonomy

The block type taxonomy is canonical and lives in the substrate. Initial types (extensible):

- `EntityHeader` — entity name, primary attributes, status, trust band
- `ClaimSummary` — one or more claims rendered with provenance and trust
- `ActionList` — open commitments, decisions, callouts requiring user attention
- `BriefingHeader` — date, scope, summary line, sources count
- `MagazineEnd` — finite-ending marker (per DailyOS magazine-not-dashboard discipline)
- `AgenticPrompt` — invokable agentic block (per ADR-0129 §6) with question, default ability, optional scope
- `Reflection` — claim about the user's own pattern (per longitudinal threading work)
- `Custom(String)` — extension point for type registration by abilities

New block types are added by registering them with the substrate's block-type registry. Surfaces that don't know a `Custom` block type render a **privacy-rendered fallback**, not raw payload disclosure. The fallback uses the substrate's embed model ([ADR-0074](0074-vector-search-entity-content.md), [ADR-0078](0078-nomic-embed-text-model-switch.md)) to find the nearest known `BlockType` and render the payload in that shape, with a "rendered as nearest known type" indicator. Raw payload exposure is forbidden because ability metadata is model-facing and browser-facing API surface (per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.6); unknown block types are an info-disclosure vector otherwise. (Fallback policy added 2026-05-10 per Custom block fallback finding in DOS-546 L0 Cycle 2.)

### 4. Surface bindings — renderers, not authors

Each surface ships a renderer that maps `BlockType` to its native rendering technology:

| Surface | Renderer | Maps `Block` to |
|---|---|---|
| Tauri React magazine | `ReactBlockRenderer` | React components per BlockType |
| WordPress (via SurfaceClient) | `GutenbergBlockRenderer` | Gutenberg blocks per BlockType |
| MCP head (headless) | `JsonBlockRenderer` | Claim-shaped JSON per ADR-0128 §4 |
| CLI head | `MarkdownBlockRenderer` | Markdown per ADR-0128 §7 |
| Future surfaces | Their own renderers | Per BlockType |

Renderers are surface-side code. They do not modify the composition; they project it. The same `Composition` can render in any surface and the user gets equivalent intelligence with surface-appropriate visual treatment.

Surfaces MAY decline to render specific block types (e.g., MCP head returns claim-JSON for `AgenticPrompt` rather than rendering an invocable UI; CLI head emits a marker for `MagazineEnd` rather than visual spacing). The Composition stays canonical; surfaces decide what to show.

### 5. Authorship boundary — abilities produce compositions

Abilities that produce composed pages (briefings, entity pages, prep packs, reports) return `Composition` as their output. The composition is authored by the ability with full provenance attribution per [ADR-0105](0105-provenance-as-first-class-output.md). Surfaces never construct compositions; they receive them through the normal ability-invocation path and render them.

Specifically:

- A `briefing.daily` ability produces a `Composition { kind: Briefing, sections: [...], ... }`.
- An `entity.account_overview` ability produces a `Composition { kind: EntityPage, subject: Some(account_ref), ... }`.
- A `prep.meeting` ability produces a `Composition { kind: PrepPack, subject: Some(meeting_ref), ... }`.
- WordPress's Gutenberg blocks render those compositions; they do not invent them.

This is the cleanest expression of the AI-native principle from [PHILOSOPHY.md](../../design/product/PHILOSOPHY.md): AI produces, users consume. Authorship is substrate-side; consumption is surface-side; the contract between them is the `Composition` model.

### 6. SurfaceClient consumption

A `SurfaceClient` instance (per [ADR-0111](0111-surface-independent-ability-invocation.md) §8) consumes compositions by:

1. Invoking the relevant composition-producing ability (subject to its scope grants).
2. Receiving a `Composition` instance with full provenance.
3. Rendering it via its surface-specific renderer.
4. Honoring `Salience` for visibility/ordering decisions where the surface allows differential treatment (e.g., hiding `Background` blocks in compact mode).
5. Preserving `claim_refs` so user actions on a block (dismissal, correction) round-trip as typed feedback events naming the claim and field path.

The same composition rendered by the WordPress SurfaceClient, Tauri React surface, and MCP head produces equivalent intelligence — different paint, same content, same provenance.

### 7. On-the-fly composition

Per [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) §6, layouts can be agent-composed at runtime based on substrate salience rather than fixed templates. Under this contract, on-the-fly composition is an ability behavior: the producing ability emits a `Composition` whose `sections` and `blocks` are chosen at invocation time based on what's salient. The contract is unchanged; the ability's logic does the choosing.

Templated layouts are a special case: the ability emits compositions whose section/block shape matches a predeclared template. Both modes use the same `Composition` model.

### 8. What this ADR does NOT decide

- **The full BlockType registry.** Initial types are named; the registry is extensible. New types are added through normal ability-shipping work.
- **Renderer implementations for each surface.** Out of scope for this ADR; lives in surface-specific implementation work (e.g., DOS-546 validates the WP renderer).
- **Editing semantics.** A user editing a block's value (claim correction) routes through the existing feedback path per [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §5; this ADR does not redesign feedback.
- **Persistence of compositions.** Whether the substrate stores composed outputs or regenerates them on demand is an implementation question; the contract is the same either way.
- **The composition lifecycle (versioning, invalidation, refresh).** Lives in the existing substrate work on claim lifecycle and signal propagation; compositions inherit those primitives via their `claim_refs`.

## Consequences

### Positive

- **Surfaces are genuinely interchangeable.** A new SurfaceClient instance (Obsidian, browser extension, mobile, future-thing) ships a renderer and consumes the same compositions. No substrate change required to add a surface.
- **The Composition Contract is the seam DOS-546 actually validates.** The spike validates that the WordPress SurfaceClient renderer can render substrate-authored compositions; it doesn't have to invent the composition model.
- **WordPress doesn't become the document model.** Gutenberg blocks are one render target. The substrate retains authorship; WordPress projects.
- **Salience-driven rendering is structurally enabled.** Per-block salience flows through the composition; surfaces honor it.
- **Provenance and trust bands flow naturally through composition.** Renderers can't strip them without rendering wrong.
- **Cross-surface intelligence parity is structural.** Same composition → equivalent intelligence in any surface.

### Negative / risks

- **Composition model is a substantial new contract.** Several types, several taxonomies. Implementation cost is real.
- **Existing compositions are implicit.** The Tauri React magazine builds composed pages today via React components, not via a `Composition` model. Migration to producing `Composition` from the substrate side is real work (likely in v1.4.2-v1.4.4 wave program).
- **BlockType registry needs governance.** Adding a `Custom` type that doesn't render gracefully in all surfaces is a real failure mode. Convention: every new BlockType ships a fallback markdown rendering.
- **MCP head's composition rendering** (per ADR-0128 §7) becomes more specified than today's "claim-shaped JSON" framing. The MCP renderer maps blocks to claim-JSON envelopes; existing MCP clients consume the envelopes as before, but the underlying contract is the Composition model.

### Neutral

- This ADR adds no runtime code. Contract only. Implementation lives in the v1.4.2+ wave program.
- ADR-0129 §6 is preserved and generalized — block-shaped composition is the right *generic* shape; Gutenberg is one *implementation*.
- ADR-0128 §1 is preserved — substrate is the product, surfaces are heads over it.

## References

Internal:

- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities runtime contract (composition-producing abilities use this)
- [ADR-0105](0105-provenance-as-first-class-output.md) — Provenance envelope used in compositions
- [ADR-0108](0108-provenance-rendering-and-privacy.md) — Provenance rendering per surface/actor
- [ADR-0111](0111-surface-independent-ability-invocation.md) — Surface-independent ability invocation; §8 SurfaceClient
- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS; MCP as co-equal surface
- [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) — Composable surfaces; this ADR generalizes §6's block-shaped composition claim
- [DOS-546](https://linear.app/a8c/issue/DOS-546) — WordPress Studio surface viability spike; validates the WP renderer against this contract
- [Product Philosophy](../../design/product/PHILOSOPHY.md) — AI-native: AI produces, users consume; this ADR encodes the producer/consumer boundary
- [Product Thesis](../../design/product/PRODUCT-THESIS.md) — Personal intelligence that compounds across surfaces

External:

- [WordPress Abilities API](https://developer.wordpress.org/news/2025/11/introducing-the-wordpress-abilities-api/) — One concrete renderer target for the Composition Contract
