# ADR-0130 — Surface-Independent Composition Contract

**Status:** Proposed
**Date:** 2026-05-10
**Amended:** 2026-05-10 — added explicit references to existing substrate retrieval primitives (ADR-0074 vector search, ADR-0078 embed model) in §2 (Salience computation) and §3 (Custom block fallback) to prevent composition-producing abilities from reinventing retrieval.
**Amended:** 2026-05-10 — §2 `Block.provenance` retyped from `ProvenanceEnvelope` (envelope copy) to `ProvenanceRef` (compact reference into the canonical envelope on `AbilityOutput<Composition>`). Preserves the ADR-0105 "lives once" invariant and keeps block-heavy compositions under ADR-0108's 64KB serialized-provenance cap. Source: Phase 0 artifact `06-composition-provenance-ref.md`.
**Amended:** 2026-05-10 — §3 fallback rewritten from embed-model nearest-known + "rendered as nearest known type" indicator to deterministic schema-bounded projection at JSON-Pointer granularity. Unknown payload fields are dropped, not displayed; `claim_refs` and `provenance_ref` are preserved; rendered block degrades to `needs_verification` and carries a non-dismissible banner. Source: Phase 0 artifact `07-custom-block-fallback-projection.md`.
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

#### Consumed substrate primitives

The Composition contract is a producer/consumer seam on top of existing v1.4.0/v1.4.1 substrate. Composition-producing abilities and surface renderers CONSUME the following primitives; they do not reinvent them:

- **Retrieval and salience** — Hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)) over the embed model ([ADR-0078](0078-nomic-embed-text-model-switch.md)). The substrate owns the relevance question. The Composition contract names `Salience.weight` as a producer field; the score itself flows from the retrieval primitives.
- **Trust scoring** — Per-claim trust assessment per [ADR-0105](0105-provenance-as-first-class-output.md). Renderers surface trust bands from the resolved provenance envelope; they do not compute trust.
- **Signals + invalidation** — Signal propagation per [ADR-0080](0080-signal-intelligence-architecture.md) drives composition refresh. Compositions are derived state over claims; signal-driven invalidation is the canonical refresh path.
- **Abilities runtime** — Producer-side execution per [ADR-0102](0102-abilities-as-runtime-contract.md). Composition-producing abilities are normal abilities returning `AbilityOutput<Composition>`.
- **Ability invocation** — Surface-independent invocation per [ADR-0111](0111-surface-independent-ability-invocation.md) §8. The `SurfaceClient` actor class is the invocation channel for third-party local surfaces consuming compositions.

This rule prevents the failure mode where an ability author rebuilds retrieval, trust, signals, or invocation plumbing from scratch without realizing the substrate already has it. New work CONSUMES the named primitive; it does not duplicate it.

### 2. Composition primitives

The substrate emits composed output as instances of a typed `Composition` model. A `Composition` is the `T` inside `AbilityOutput<Composition>` per [ADR-0102](0102-abilities-as-runtime-contract.md) §6 — the canonical `Provenance` envelope lives exactly once on the wrapper, not on the domain output. The Composition model itself carries no envelope copy; blocks carry compact references into the wrapper envelope. The model:

```rust
pub struct Composition {
    pub id: CompositionId,
    pub kind: CompositionKind,  // Briefing | EntityPage | PrepPack | Report | Callout | Custom
    pub subject: Option<EntityRef>,  // The entity this composition is about, if any
    pub sections: Vec<Section>,
    pub salience: Salience,         // Top-level relevance weight
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
    pub provenance: ProvenanceRef, // Compact reference into AbilityOutput<Composition>.provenance
    pub salience: Salience,
    pub render_hints: RenderHints,  // Surface-neutral hints (emphasis, density, etc.)
}

/// Reference into the canonical provenance envelope that lives once on
/// AbilityOutput<Composition>.provenance per ADR-0102 §6 and ADR-0105 §8.
/// Renderers resolve this against the ability's top-level envelope; they
/// never embed a copy of the envelope on the block.
pub struct ProvenanceRef {
    pub invocation_id: InvocationId,  // Identifies the canonical envelope
    pub field_path: FieldPath,        // JSON Pointer into the envelope's field_attributions
}

pub struct Salience {
    pub weight: f32,           // 0.0 - 1.0
    pub band: SalienceBand,    // critical | important | contextual | background
    pub reason: String,        // Why this is at this weight
}

pub enum SalienceBand { Critical, Important, Contextual, Background }
```

**Why `ProvenanceRef`, not `ProvenanceEnvelope`.** Embedding a full envelope on every block creates a second provenance carrier inside the domain output, which violates [ADR-0102](0102-abilities-as-runtime-contract.md) §6 + §9 Rule 5 (provenance lives exactly once on `AbilityOutput<T>`). It also defeats [ADR-0108](0108-provenance-rendering-and-privacy.md)'s 64KB serialized-provenance cap: a 24-block composition with typical 12KB Transform envelopes serializes to ~288KB of duplicated provenance, 4.5× the cap before any block content is counted. A `ProvenanceRef` is ~80-200 bytes serialized; a 60-block long-form composition spends ~13KB on block refs, well inside the cap. The reference shape mirrors [ADR-0105](0105-provenance-as-first-class-output.md) §8's planned-mutation pattern — plans reference the envelope by invocation id and field path rather than duplicating it.

**Resolution.** Renderers resolve a `ProvenanceRef` by fetching the canonical envelope for `invocation_id` from the runtime provenance store (already addressable per ADR-0105/ADR-0108), reading the `FieldAttribution` at `field_path`, and passing the resolved envelope through [ADR-0108](0108-provenance-rendering-and-privacy.md) §2's actor-filtered `render_provenance_for(prov, actor, surface)`. Resolution can fail (garbage-collected envelope, unknown invocation across environments, unauthorized actor, masked provenance, missing field attribution); failure MUST degrade visibly — the block renders with a provenance-unavailable banner or downgraded trust band, never as fully trusted. When the envelope resolves but the exact `field_path` does not, the surface MAY fall back to invocation-level provenance, but MUST label the fallback as less specific.

**Block-builder validation.** Block constructors MUST validate that `field_path` resolves into `output.provenance.field_attributions` at construction time, or fail. This prevents accidental broad attribution.

**Composition-size guard.** Block provenance refs SHOULD stay under 48KB serialized across a single composition. Pathologically block-heavy compositions (>~200 blocks) that exceed the guard MUST collapse to section-level refs rather than embedding envelopes. The 48KB guard preserves headroom under ADR-0108's 64KB cap for surrounding composition metadata.

Provenance and trust bands still flow through the composition at the block level, not as a footer. A renderer that strips provenance refs (or resolves them and discards trust treatment) is rendering wrong; the contract treats provenance as part of the composition's meaning, not its decoration.

Source: Phase 0 artifact [`06-composition-provenance-ref.md`](../plans/dos-546/phase-0/06-composition-provenance-ref.md).

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

New block types are added by registering them with the substrate's block-type registry.

#### 3.1 Custom block fallback projection

Surfaces that do not know a `Custom` block type MUST render a **schema-bounded projection** onto the nearest known block type. Renderers MUST NOT render unknown block payload fields directly. The previous "raw payload available" fallback is removed — an unknown block type is a privacy boundary, not a debugging opportunity. Unknown payload fields may include internal notes, source excerpts, email addresses, prompt context, debug carriers, or fields whose sensitivity tier is not allowed on the current surface (per [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)). Ability metadata is also model-facing and browser-facing API surface per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.6; raw payload disclosure is an info-disclosure vector.

When a renderer encounters a block whose `block_type` is not in the renderer's recognized type set, it MUST apply the following deterministic algorithm:

1. **Select the nearest known type deterministically.** Rank candidate known block types by (a) matching `composition_kind` (weight 100), (b) compatible required-pointer overlap (weight 10 per pointer), (c) compatible optional-pointer overlap (weight 2 per pointer), (d) render-annotation similarity (0-20), (e) namespace-prefix similarity (0-5). Break ties by `kind_match`, then `required_overlap`, then `optional_overlap`, then `annotation_similarity`, then **lexicographic block type id**. The lexicographic final tie-break guarantees all renderers across all surfaces select the same fallback for the same input. Candidate eligibility is gated by `allowed_surfaces` and actor reachability.
2. **Compute the JSON-Pointer intersection** between the unknown block's declared schema and the selected nearest-known type's schema. A pointer is eligible only when it appears in both schemas and the field shapes are compatible under safe-widening rules (integer→number, enum-member→display-string only when annotated as display text; object→string, array→string, arbitrary-JSON→string are unsafe and forbidden).
3. **Project only intersected pointers** from the unknown payload into a new payload for the nearest-known type. Reconstruct container objects as needed to hold allowed leaves; do not copy sibling objects wholesale. Array projection rebuilds each item with only intersected item pointers.
4. **Drop every non-intersected payload field.** This includes fields with identical names whose pointer does not appear in the selected type's schema. Dropped values MUST NOT appear in visible text, `data-*` attributes, HTML comments, serialized block attributes, REST preload state, hydration state, inspector UI, debug panels, logs, or diagnostics. Diagnostics MAY include pointer counts and non-sensitive type ids; they MUST NOT include dropped payload values.
5. **Preserve `claim_refs` exactly.** These are not payload — they are stable references into the claim substrate and are required for downstream link resolution.
6. **Preserve `provenance` (the `ProvenanceRef`) exactly.** This is not payload — it is a reference into the [ADR-0105](0105-provenance-as-first-class-output.md) envelope and renders only through [ADR-0108](0108-provenance-rendering-and-privacy.md) actor-filtered rules. The fallback path MUST NOT dereference `claim_refs` or `provenance` to backfill dropped payload fields.
7. **Render as the selected nearest-known type** using the projected payload, preserved `claim_refs`, and preserved `provenance`.
8. **Show a non-dismissible banner on the block:** `Rendered as <nearest-known-type> — payload may be incomplete`. The banner uses product-facing vocabulary; internal terms (`enrichment`, `intelligence pipeline`, `substrate`, `schema projection`) MUST NOT appear in user-visible banner copy.
9. **Cap the rendered block's trust band at `needs_verification`.** Fallback MUST NOT upgrade trust. If the original would have rendered at a stronger band, the cap applies; if it was already `needs_verification`, it stays.

If no candidate has a positive deterministic score, or if the selected candidate has no eligible schema intersection with the unknown schema, the renderer MUST render the block as generic `dailyos/text` with an empty payload, preserved `claim_refs`, preserved `provenance`, the banner `Rendered as dailyos/text — payload may be incomplete`, and trust band `needs_verification`.

**Schema source for the unknown type.** The unknown block schema MUST come from the composition manifest, the block registry, or a versioned schema bundle shipped with the composition. Renderers MUST NOT infer display-allowed fields from the raw payload. If no schema is available for the unknown type, the unknown schema is treated as empty and the generic `dailyos/text` fallback applies.

**Substrate-side count guard.** The substrate SHOULD cap unknown-block count per composition (the exact threshold is a producer-side guard, not a renderer concern). A composition dominated by unknown blocks is a producer bug, not a normal fallback case.

Source: Phase 0 artifact [`07-custom-block-fallback-projection.md`](../plans/dos-546/phase-0/07-custom-block-fallback-projection.md). (Fallback policy added 2026-05-10 per Custom block fallback finding in DOS-546 L0 Cycle 2; rewritten 2026-05-10 per /cso refinement 10 to schema-bounded projection.)

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
2. Receiving an `AbilityOutput<Composition>` — the `Composition` is the domain output; the canonical `Provenance` envelope lives once on the wrapper.
3. Rendering blocks via its surface-specific renderer. For each block, resolve `provenance: ProvenanceRef` against the wrapper envelope. Batch lookups by unique `invocation_id` on first paint to avoid per-block round trips; the first visible viewport SHOULD complete within ~50ms p95 for local-runtime lookups and ~150ms p95 for remote-runtime lookups. Content MAY render immediately with a pending trust band when the lookup is slower.
4. Honoring `Salience` for visibility/ordering decisions where the surface allows differential treatment (e.g., hiding `Background` blocks in compact mode).
5. Preserving `claim_refs` so user actions on a block (dismissal, correction) round-trip as typed feedback events naming the claim and field path.
6. Applying [ADR-0108](0108-provenance-rendering-and-privacy.md) §2 `render_provenance_for(prov, actor, surface)` to every resolved envelope. The `ProvenanceRef` does not bypass actor/surface filtering; it only locates the canonical envelope to which those rules apply.

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
- **Provenance lives once.** `ProvenanceRef` preserves ADR-0102 §6 + ADR-0105 §8's "lives once" invariant. Block-heavy compositions stay inside ADR-0108's 64KB serialized-provenance cap — a 60-block long-form composition spends ~13KB on refs vs. ~720KB if envelopes were copied per block.
- **Unknown blocks are a privacy boundary, not a debug surface.** Schema-bounded fallback projection prevents internal notes, source excerpts, email addresses, prompt context, and debug carriers from reaching the DOM, REST preload state, hydration state, or inspector UI via unknown payload fields. Sparse-but-safe is the correct behavior for unknown content.
- **Cross-surface intelligence parity is structural.** Same composition → equivalent intelligence in any surface, with the same deterministic fallback selection (lexicographic tie-break) when a renderer encounters an unknown type.

### Negative / risks

- **Composition model is a substantial new contract.** Several types, several taxonomies, plus the `ProvenanceRef` resolution and schema-bounded projection paths. Implementation cost is real.
- **Existing compositions are implicit.** The Tauri React magazine builds composed pages today via React components, not via a `Composition` model. Migration to producing `Composition` from the substrate side is real work (likely in v1.4.2-v1.4.4 wave program).
- **BlockType registry needs governance.** Every new block type ships with a JSON Schema (required + optional pointer sets, render annotations, allowed surfaces, default trust band) so fallback projection has a deterministic target. A type that ships without a schema is unrenderable by foreign surfaces.
- **Provenance-resolver retention.** Compositions that outlive runtime provenance retention need either longer retention for published-composition invocations or a durable exported provenance bundle keyed by `invocation_id`. The block ref shape is the right address either way; what changes is where the envelope is fetched from.
- **MCP head's composition rendering** (per ADR-0128 §7) becomes more specified than today's "claim-shaped JSON" framing. The MCP renderer maps blocks to claim-JSON envelopes; existing MCP clients consume the envelopes as before, but the underlying contract is the Composition model.

### Neutral

- This ADR adds no runtime code. Contract only. Implementation lives in the v1.4.2+ wave program.
- ADR-0129 §6 is preserved and generalized — block-shaped composition is the right *generic* shape; Gutenberg is one *implementation*.
- ADR-0128 §1 is preserved — substrate is the product, surfaces are heads over it.
- ADR-0102 §6/§9 Rule 5 and ADR-0105 §8 are preserved structurally — provenance lives once, on the wrapper, and references address it.

## References

Internal:

- [ADR-0074](0074-vector-search-entity-content.md) — Hybrid vector search; substrate retrieval primitive consumed for salience
- [ADR-0078](0078-nomic-embed-text-model-switch.md) — Embed model backing vector search
- [ADR-0080](0080-signal-intelligence-architecture.md) — Signal propagation; canonical refresh path for composition-derived state
- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities runtime contract; §6 + §9 Rule 5 establish provenance-lives-once on `AbilityOutput<T>`; §7.1 canonical AbilityPolicy schema; §7.6 governs SurfaceClient-reachable ability metadata
- [ADR-0105](0105-provenance-as-first-class-output.md) — Provenance envelope shape; §8 establishes the reference pattern that `ProvenanceRef` mirrors
- [ADR-0108](0108-provenance-rendering-and-privacy.md) — Actor-filtered provenance rendering; 64KB serialized-provenance cap that motivates the ref shape
- [ADR-0111](0111-surface-independent-ability-invocation.md) — Surface-independent ability invocation; §8 `SurfaceClient` is the consumer for the Composition Contract
- [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md) — Sensitivity tiers (`Public`, `Internal`, `Confidential`, `UserOnly`) consumed by schema-bounded fallback projection
- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS; MCP as co-equal surface
- [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) — Composable surfaces; this ADR generalizes §6's block-shaped composition claim; WP Studio is the first concrete renderer
- [Phase 0 artifact 06](../plans/dos-546/phase-0/06-composition-provenance-ref.md) — Source for the `ProvenanceRef` shape (§2 amendment)
- [Phase 0 artifact 07](../plans/dos-546/phase-0/07-custom-block-fallback-projection.md) — Source for schema-bounded fallback projection (§3.1 amendment)
- [DOS-546](https://linear.app/a8c/issue/DOS-546) — WordPress Studio surface viability spike; validates the WP renderer against this contract
- [Product Philosophy](../../design/product/PHILOSOPHY.md) — AI-native: AI produces, users consume; this ADR encodes the producer/consumer boundary
- [Product Thesis](../../design/product/PRODUCT-THESIS.md) — Personal intelligence that compounds across surfaces

External:

- [WordPress Abilities API](https://developer.wordpress.org/news/2025/11/introducing-the-wordpress-abilities-api/) — One concrete renderer target for the Composition Contract
