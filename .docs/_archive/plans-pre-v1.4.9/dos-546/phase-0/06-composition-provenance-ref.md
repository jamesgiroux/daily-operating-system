---
status: spec:ready
date: 2026-05-10
amends_adr: 0130
related_adrs: [0102, 0105, 0108]
open_questions: see ./INDEX.md (routed to W0-A and W1-F L0 Prep)
---

# Composition Provenance Reference Shape — ADR-0130 Amendment Draft

## Context

This is a Phase 0 design artifact for DOS-546, the DailyOS WordPress Studio
surface viability spike.

The target amendment is ADR-0130, "Surface-Independent Composition Contract."

The current ADR-0130 §2 `Block` shape is reported to include:

```rust
pub provenance: ProvenanceEnvelope
```

This draft proposes replacing that field with:

```rust
pub provenance: ProvenanceRef
```

The amendment aligns the composition contract with the existing provenance
contract in ADR-0102, ADR-0105, and ADR-0108.

ADR-0102 establishes the abilities layer as the runtime contract for
synthesized user-facing and agent-facing outputs.

ADR-0102 §1 says an ability returns a typed output wrapped in
`AbilityOutput<T>`, carrying domain data alongside a mandatory provenance
envelope.

ADR-0102 §6 is stricter: the output is always wrapped in `AbilityOutput<T>`,
which carries provenance exactly once.

ADR-0102 §6 also states that domain output types do not re-embed provenance.

ADR-0102 §9 Rule 5 repeats the invariant: every ability output carries
provenance via `AbilityOutput<T>`, and provenance lives exactly once, on the
wrapper.

ADR-0105 defines the `Provenance` envelope shape.

ADR-0105 §1 gives each envelope an `invocation_id`, `ability_name`,
`ability_version`, `ability_schema_version`, temporal context, trust
assessment, sources, child provenance, field-level attribution, and warnings.

ADR-0105 §5 requires every field in an ability output to have a
field-level attribution keyed by a `FieldPath`.

ADR-0105 §6 defines composition semantics: when ability A invokes ability B,
A's provenance contains B's provenance as a nested child.

ADR-0105 §8 already uses a reference pattern for planned mutations:
plans do not duplicate provenance; they reference the envelope by invocation
ID and field path.

ADR-0108 defines how surfaces render provenance safely.

ADR-0108 §1 says the app renders an "About this" affordance, summary,
details, and full provenance on demand.

ADR-0108 §2 centralizes rendering through `render_provenance_for(prov, actor,
surface)`.

ADR-0108 §5 sets rendering budgets, including a top-level app summary budget
of 2KB and MCP default response budget of 10KB.

ADR-0108's 2026-04-20 amendment adds a hard cap: serialized provenance is
limited to 64KB per ability output.

The cap applies before surface rendering.

Rendering truncation cannot save an over-cap envelope, because the envelope
has already failed construction.

The WordPress Studio surface should therefore treat composition blocks as
surface projection records, not new canonical provenance carriers.

The canonical provenance remains the `AbilityOutput<T>.provenance` envelope
produced by the runtime.

Composition blocks carry references into those canonical envelopes.

This draft could not verify the exact ADR-0130 text in this checkout because
`.docs/decisions/0130-surface-independent-composition-contract.md` is not
present.

It therefore writes a replacement §2 that is self-contained and designed to
drop into the reported ADR-0130 shape.

## Problem with Current Shape

Embedding `ProvenanceEnvelope` directly on every `Block` creates a second
provenance carrier inside the domain output.

That conflicts with ADR-0102's core invariant.

ADR-0102 requires provenance to live exactly once on `AbilityOutput<T>`.

The domain output is allowed to contain fields that the user or surface needs.

The domain output is not allowed to re-declare a `provenance` field containing
the full envelope.

ADR-0130 compositions are ability outputs when they are synthesized or
composed artifacts.

A `Composition` is therefore the `T` inside `AbilityOutput<Composition>`.

If `Composition.blocks[].provenance` embeds a full envelope, the domain output
now contains many copies of provenance in addition to the canonical wrapper
provenance.

That breaks the "exactly once" rule structurally, not just stylistically.

The shape also creates ambiguity.

If a block's embedded envelope disagrees with the wrapper envelope, renderers
must choose one.

If the embedded envelope is stale after source masking, renderers could show
revoked source detail even though the canonical envelope was masked.

If a composition block is copied, split, reordered, or transformed for a
surface, the embedded envelope can become separated from the output-time
invocation that produced it.

Those are avoidable problems.

The canonical envelope already has identity, field-level attribution, and
composition-tree context.

A block only needs to say which invocation and which field or claim in that
invocation supports the block.

Embedding the full envelope also risks ADR-0108's 64KB cap.

The cap applies to serialized provenance per ability output.

A block-heavy composition that stores the full envelope per block repeats the
same ability identity, source list, child tree, field attribution map, warnings,
and prompt fingerprint over and over.

The repeated data does not add trust value.

It only expands the payload.

Concrete size math:

Assume a typical composed Transform provenance envelope is 12KB.

That is conservative relative to ADR-0105's range: Read ability provenance is
roughly 500B to 5KB, and Transform provenance with field attributions can
reach roughly 50KB.

Assume a typical WordPress Studio composition contains 24 blocks.

That is plausible for a publishable artifact: title block, intro paragraph,
three section headings, nine body paragraphs, three pull quotes, four claim or
stat blocks, one CTA, one metadata block, and two footer or attribution blocks.

With embedded envelopes:

```text
24 blocks * 12KB envelope = 288KB repeated provenance
```

That is 4.5x the 64KB cap before counting any block content.

With a heavier Transform envelope:

```text
24 blocks * 35KB envelope = 840KB repeated provenance
```

That is more than 13x the cap.

Even a small 4KB Read-style envelope fails for block-heavy output:

```text
24 blocks * 4KB envelope = 96KB repeated provenance
```

That exceeds the cap before the composition stores titles, body text, layout
hints, target surface metadata, or diagnostics.

The repeated-envelope shape therefore makes normal compositions either
non-conforming or dependent on truncation before the surface can render them.

Truncation is the wrong answer here.

The problem is not that the provenance is too detailed.

The problem is that the same canonical provenance is being duplicated at the
wrong layer.

## ProvenanceRef Shape

Two candidate shapes were considered.

Candidate A:

```rust
pub struct ProvenanceRef {
    pub invocation_id: InvocationId,
    pub field_path: JsonPointer,
}
```

Candidate B:

```rust
pub struct ProvenanceRef {
    pub claim_id: ClaimId,
    pub field_path: JsonPointer,
}
```

This draft chooses Candidate A.

Candidate A references the ability invocation that produced the block and a
JSON Pointer into that invocation's provenance envelope or output field
attribution map.

The reference preserves the full ability chain.

A renderer can resolve the invocation to the canonical `Provenance` envelope,
then use `field_path` to find the relevant `FieldAttribution`.

From there, the renderer can show ability name, ability version, produced time,
trust assessment, direct sources, child abilities, warnings, and any
field-level explanation allowed by ADR-0108.

Candidate A aligns with ADR-0105 §8.

ADR-0105 already says planned mutations reference an ability's provenance by
invocation ID and field path to prevent duplication.

Composition blocks should use the same pattern unless there is a stronger
reason to diverge.

Candidate A also aligns with ADR-0108.

ADR-0108 already names `get_provenance(invocation_id)` as the detail-fetch
path for authorized MCP agents.

Surface clients can share that invocation-resolution model.

Candidate B has one attractive property: `claim_id` may be stable across
ability re-invocation.

That matters if a block is meant to bind to a durable claim independent of the
specific run that selected or rendered it.

However, Candidate B loses context that the surface needs for trust rendering.

A claim can tell the renderer what claim was used.

It cannot, by itself, tell the renderer which ability selected that claim,
which transform composed it with other evidence, which prompt fingerprint was
used, what sources were direct versus transitive for this output, or whether
the output was degraded.

Those details live in the invocation envelope.

The WordPress Studio surface is not only rendering stored claims.

It is rendering a composed artifact produced by an ability.

For that surface, the relevant provenance question is not only "what claim is
this based on?"

The relevant question is "what ability invocation produced this block, from
which sources and child abilities, under what trust classification?"

Candidate A answers that.

Candidate B can still be represented inside Candidate A.

If the field attribution references a durable claim, the resolved provenance
can expose that claim through `SourceRef`, `SourceAttribution`, or a
claim-specific output field.

The block does not need to make the claim ID the primary provenance address.

Chosen shape:

```rust
pub struct ProvenanceRef {
    pub invocation_id: InvocationId,
    pub field_path: JsonPointer,
}
```

`invocation_id` identifies the canonical `AbilityOutput<T>.provenance`
envelope.

`field_path` identifies the field in the ability output that the block derives
from.

`field_path` uses JSON Pointer syntax.

For block-level display, `field_path` should usually point at the output field
that directly produced the block, such as `/blocks/7/content`,
`/sections/2/claims/1`, or `/body/paragraphs/4`.

When the output structure contains stable IDs, stable path segments should be
preferred over raw array positions where the local schema supports them.

The field path is not a claim locator.

It is a pointer into the invocation's output/provenance attribution space.

The renderer resolves that pointer through the canonical envelope's
`field_attributions` map.

If ADR-0130 wants the field to remain named `provenance` for ergonomic reasons,
the type should still be `ProvenanceRef`.

An alternative field name, `provenance_ref`, is clearer and matches
ADR-0104's planned mutation shape.

This draft recommends `provenance_ref` in new code but keeps the drop-in text
compatible with either naming decision.

## Renderer Resolution

The WordPress renderer, Tauri renderer, MCP renderer, and any future
`SurfaceClient` resolve a `ProvenanceRef` through the same conceptual steps.

Step 1: collect refs needed for the first paint.

The surface scans the composition blocks visible in the initial viewport.

For each visible block that has a `ProvenanceRef`, the surface extracts
`invocation_id`.

The surface deduplicates by `invocation_id`.

If 12 visible blocks all point to the same invocation, the surface performs one
envelope lookup, not 12.

Step 2: resolve canonical envelopes.

The surface calls the runtime provenance store with the unique invocation IDs.

The runtime returns the canonical `Provenance` envelope for each invocation.

The canonical location is runtime-side storage of ability outputs, keyed by
`invocation_id`.

At minimum, this is the same operational provenance store described by
ADR-0105 and ADR-0108.

For ability outputs still in memory during a request, the runtime may also
serve from the current invocation result cache.

The canonical envelope is not stored on the block.

The block stores only the address.

Step 3: resolve field attribution.

For each block, the surface takes its resolved envelope and reads:

```text
envelope.field_attributions[block.provenance_ref.field_path]
```

If present, that field attribution is the block's display-specific provenance
entry point.

The renderer then follows `source_refs` to direct sources or child provenance
entries.

If the path points at a field without an exact attribution entry, the resolver
may walk up to the nearest parent path.

For example, if `/blocks/7/content` is missing but `/blocks/7` exists, the
resolver can use `/blocks/7` and mark the result as less specific.

The fallback must be visible to diagnostics.

It must not silently pretend the exact field was attributed.

Step 4: actor-filter the result.

The surface passes the resolved provenance and selected field attribution to
the shared renderer:

```rust
render_provenance_for(&provenance, actor, surface)
```

The renderer applies ADR-0108 actor and surface rules.

A WordPress Studio preview shown to the owning user can show source names and
detail appropriate to the first-party app.

An MCP response is filtered for `Actor::Agent`.

A P2 publication shows only the heavily redacted external footnote.

The `ProvenanceRef` does not bypass those rules.

It only locates the canonical envelope to which those rules apply.

Step 5: render the trust UI.

The surface displays the summary required by its surface contract:

ability name, produced timestamp, source count, trust assessment, composition
depth, warnings, and field-level attribution where appropriate.

The top-level summary must remain within ADR-0108's rendering budget.

The renderer can fetch full detail on expansion.

Canonical storage:

The runtime owns canonical provenance.

Canonical provenance is addressable by `invocation_id`.

The runtime may expose this as:

```rust
pub trait ProvenanceResolver {
    async fn get_provenance(
        &self,
        invocation_id: InvocationId,
        actor: Actor,
        surface: Surface,
    ) -> Result<ProvenanceOrMasked, ProvenanceResolveError>;
}
```

The resolver should return the canonical envelope before rendering, or a masked
variant if source revocation rules require masking.

The resolver should not return block-local copies.

Caching strategy:

Provenance envelopes are immutable per invocation.

Once an `invocation_id` is minted, the envelope content for that invocation
does not change except for privacy masking caused by source revocation.

That means clients can cache aggressively by `invocation_id` and actor/surface
filter.

Recommended cache layers:

1. Request-local cache in the renderer, keyed by `InvocationId`.
2. Surface-session cache in the `SurfaceClient`, keyed by
   `(InvocationId, Actor, Surface)`.
3. Runtime cache for recently produced ability outputs, keyed by
   `InvocationId`.

Cache invalidation:

Source revocation and provenance masking must invalidate or version affected
entries.

If the runtime can return a `masked_at` or `provenance_revision` marker, the
client can distinguish an immutable original from a privacy-masked replacement.

Absent that marker, the conservative strategy is to keep client caches
session-scoped and clear them when source connection or revocation state
changes.

Failure behavior:

Resolution can fail.

Examples:

1. The envelope was garbage-collected under retention policy.
2. The invocation ID is unknown because the composition was imported from
   another environment.
3. The current actor is not authorized to see that provenance.
4. The field path no longer exists in the envelope's attribution map.
5. The envelope has been masked due to source revocation.

Failure must degrade visibly.

The renderer should show a provenance-unavailable banner or block-level trust
band.

Recommended user-facing summary:

```text
Provenance unavailable for this block. The original ability invocation could
not be resolved.
```

Recommended trust treatment:

The block's trust band is degraded to "Unverified provenance" or the nearest
existing equivalent.

The surface should not display the block as fully trusted when the provenance
reference cannot resolve.

If the envelope resolves but the exact field path does not, the surface can
show invocation-level provenance with a "field attribution unavailable" warning.

If the envelope resolves to `ProvenanceMasked`, the surface follows ADR-0108's
masked rendering rules and makes clear that source details are unavailable for
privacy reasons.

Lookup latency budget:

The first paint should not wait on N per-block provenance round trips.

The initial visible viewport should issue a batched lookup for unique
invocation IDs.

Target budget:

```text
<= 50ms p95 local runtime lookup for first-viewport provenance summaries
<= 150ms p95 remote runtime lookup for first-viewport provenance summaries
```

The page can render content immediately with a pending trust band if the
lookup is slower.

The "About this" affordance can hydrate progressively.

Full provenance JSON is never required for first paint.

First paint needs only summary-sized rendered provenance, which ADR-0108 caps
at 2KB for app initial render.

## 64KB Cap Accounting

The new shape changes block accounting from repeated envelopes to small refs.

Old shape:

```text
composition provenance payload ~= N * serialized(ProvenanceEnvelope)
```

New shape:

```text
composition provenance payload ~= N * serialized(ProvenanceRef)
```

Approximate serialized sizes:

```json
{"invocation_id":"01HZY7VK9R8Y6E5B4Z3AXP4K0Q","field_path":"/blocks/7/content"}
```

That is roughly 80 to 120 bytes in compact JSON for common paths.

If IDs are UUID strings and paths are longer, assume 140 bytes.

If schema and JSON overhead are more verbose, assume 180 bytes.

Use 200 bytes as a conservative accounting value.

By contrast, ADR-0105's envelope size guidance gives:

```text
Read envelope:       ~500B to 5KB
Transform envelope:  up to ~50KB
Composed envelope:   can exceed 500KB without caps
ADR-0108 hard cap:   64KB serialized provenance per ability output
```

Practical comparison for a 24-block composition:

```text
Old, 4KB envelope:    24 * 4KB   = 96KB
Old, 12KB envelope:   24 * 12KB  = 288KB
Old, 35KB envelope:   24 * 35KB  = 840KB
New, 200B ref:        24 * 200B  = 4.8KB
```

The new shape saves:

```text
96KB - 4.8KB = 91.2KB saved in the small-envelope case
288KB - 4.8KB = 283.2KB saved in the typical Transform case
840KB - 4.8KB = 835.2KB saved in the heavier Transform case
```

Worst-case composition sizing with refs:

Assume a very block-heavy Studio document with 150 blocks.

Assume a conservative 220 bytes per serialized ref.

```text
150 * 220B = 33,000B ~= 32.2KB
```

That leaves about 31KB under the 64KB provenance cap if these refs are counted
as provenance-like metadata.

A more typical 60-block long-form artifact:

```text
60 * 220B = 13,200B ~= 12.9KB
```

A compact 24-block artifact:

```text
24 * 220B = 5,280B ~= 5.2KB
```

The ref shape therefore keeps block-level provenance metadata comfortably
inside the cap for realistic compositions.

There is still a pathological case.

If a composition allows thousands of blocks, even refs can exceed 64KB.

For example:

```text
400 * 220B = 88,000B ~= 86KB
```

That should be handled by a separate composition-size guard, not by embedding
envelopes.

Recommended guard:

```text
Composition block provenance refs SHOULD stay under 48KB serialized.
If refs exceed 48KB, the composition builder SHOULD fail or collapse blocks
into section-level provenance refs.
```

The 48KB guard preserves headroom for surrounding composition metadata,
diagnostics, and future fields if refs are included in the ability output's
serialized envelope budget calculations.

The canonical `AbilityOutput<Composition>.provenance` still has its own 64KB
cap.

The block refs do not replace that envelope.

They make the domain output point back into it.

That distinction matters:

1. The ability output still has a canonical envelope for the composition as a
   whole.
2. Each block can still render field-level provenance.
3. The block does not duplicate the canonical envelope.
4. The renderer can fetch detail on demand without bloating first paint.

## Migration Path

Existing `AbilityOutput<T>` producers do not change.

They continue to produce:

```rust
pub struct AbilityOutput<T> {
    pub data: T,
    pub provenance: Provenance,
    pub ability_version: AbilityVersion,
    pub diagnostics: Diagnostics,
}
```

Existing provenance builders do not need to change their envelope shape.

They already produce `invocation_id`.

They already populate `field_attributions`.

They already support child provenance through composition.

The change is isolated to composition block construction.

Block constructors must accept or derive a `ProvenanceRef` instead of copying a
full envelope.

Recommended helper API:

```rust
impl ProvenanceRef {
    pub fn from_output_field<T>(
        output: &AbilityOutput<T>,
        field_path: impl Into<JsonPointer>,
    ) -> Self {
        Self {
            invocation_id: output.provenance.invocation_id,
            field_path: field_path.into(),
        }
    }
}
```

Recommended block builder API:

```rust
impl BlockBuilder {
    pub fn provenance_ref(mut self, provenance_ref: ProvenanceRef) -> Self {
        self.provenance_ref = Some(provenance_ref);
        self
    }

    pub fn from_output_field<T>(
        mut self,
        output: &AbilityOutput<T>,
        field_path: impl Into<JsonPointer>,
    ) -> Self {
        self.provenance_ref = Some(ProvenanceRef::from_output_field(output, field_path));
        self
    }
}
```

Recommended validation:

The block builder should validate that `field_path` is present in
`output.provenance.field_attributions`.

If there is no exact field attribution, the builder should either fail or
require an explicit `allow_parent_attribution` call.

That prevents accidental broad attribution.

Recommended resolver helper:

```rust
pub struct ResolvedBlockProvenance {
    pub invocation_id: InvocationId,
    pub field_path: JsonPointer,
    pub rendered: RenderedProvenance,
    pub specificity: AttributionSpecificity,
}

pub enum AttributionSpecificity {
    ExactField,
    ParentField { resolved_path: JsonPointer },
    InvocationOnly,
    Unavailable,
}
```

Compatibility:

Because DOS-546 is a viability spike and ADR-0130 does not appear to be shipped
in this checkout, a flag-day swap is preferable.

The contract should change before implementation depends on embedded
envelopes.

No compatibility shim is required for production data.

If in-flight spike code already constructs `Block.provenance:
ProvenanceEnvelope`, use a temporary adapter only inside the spike branch:

```rust
impl TryFrom<(&AbilityOutput<Composition>, JsonPointer)> for ProvenanceRef {
    type Error = ProvenanceRefError;

    fn try_from(value: (&AbilityOutput<Composition>, JsonPointer)) -> Result<Self, Self::Error> {
        let (output, field_path) = value;
        if !output.provenance.field_attributions.contains_key(&field_path) {
            return Err(ProvenanceRefError::MissingFieldAttribution(field_path));
        }
        Ok(ProvenanceRef {
            invocation_id: output.provenance.invocation_id,
            field_path,
        })
    }
}
```

Do not write a long-lived reader that accepts both embedded envelopes and refs.

That would preserve the ambiguity this amendment is trying to remove.

If a temporary reader is needed for local spike artifacts, gate it behind a
development-only migration command and delete it before the contract is marked
accepted.

Rollout steps:

1. Amend ADR-0130 §2 to define `ProvenanceRef`.
2. Update any composition schema examples to use `provenance_ref`.
3. Update block constructors to call `ProvenanceRef::from_output_field`.
4. Add validation that every renderable block has either a `ProvenanceRef` or
   an explicit `ProvenanceOptional` reason for presentation-only blocks.
5. Add resolver tests for exact path, parent fallback, unknown invocation,
   masked envelope, and unauthorized actor.
6. Add size tests proving a representative block-heavy composition stays under
   the ref budget.

## Drop-In Amendment Text for ADR-0130 §2

````markdown
### 2. Block Shape

Composition blocks are surface-independent projection records.

They do not carry canonical provenance envelopes.

Canonical provenance lives exactly once on `AbilityOutput<T>.provenance` per
ADR-0102 and ADR-0105. A block carries only a lightweight reference into that
canonical envelope.

```rust
pub struct Composition {
    pub composition_id: CompositionId,
    pub schema_version: SchemaVersion,
    pub blocks: Vec<Block>,
    pub semantics: CompositionSemantics,
}

pub struct Block {
    pub block_id: BlockId,
    pub block_type: BlockType,
    pub content: BlockContent,
    pub layout: BlockLayout,
    pub provenance_ref: ProvenanceRef,
}

pub struct ProvenanceRef {
    pub invocation_id: InvocationId,
    pub field_path: JsonPointer,
}
```

`ProvenanceRef.invocation_id` points at the canonical provenance envelope for
the ability invocation that produced the block.

`ProvenanceRef.field_path` is a JSON Pointer into that invocation's output
field attribution space. The path SHOULD identify the most specific output
field that produced the block, such as `/blocks/3/content` or
`/sections/2/claims/1`. When the output schema provides stable identifiers,
stable identifier paths SHOULD be preferred over array-index-only paths.

Blocks MUST NOT embed `Provenance`, `ProvenanceEnvelope`, or any full
provenance envelope copy.

This preserves ADR-0102's invariant that provenance lives exactly once on
`AbilityOutput<T>` and prevents block-heavy compositions from multiplying the
same envelope until they exceed ADR-0108's 64KB serialized provenance cap.

The runtime keeps provenance canonical and addressable by `invocation_id`.
Surface clients resolve a block's `ProvenanceRef` by fetching the canonical
envelope for `invocation_id`, reading the `FieldAttribution` at `field_path`,
and passing the resolved envelope through ADR-0108's actor-filtered renderer.

Resolution failure is a visible trust degradation, not a silent success. If the
invocation is unknown, garbage-collected, masked, unauthorized, or missing the
requested field attribution, the surface MUST render a provenance-unavailable
state or degraded trust band for the affected block. It MAY fall back to
invocation-level provenance when the envelope exists but the exact field path
does not, provided that fallback is labeled as less specific.

Provenance refs are intentionally small. A serialized ref is expected to be
roughly 80-200 bytes, compared with typical provenance envelopes ranging from
500B-5KB for Read abilities and up to roughly 50KB for Transform abilities.
Composition builders SHOULD reject or section-collapse pathological documents
whose block provenance refs alone exceed 48KB serialized, preserving headroom
under ADR-0108's 64KB cap.
````

## Open Questions

ADR-0130 was not present in this checkout.

James should verify that the replacement text matches the actual §2 names for
`Composition`, `Block`, `BlockType`, `BlockContent`, `BlockLayout`, and
`CompositionSemantics`.

If ADR-0130 already uses `provenance` as the field name, James should decide
whether to keep the field name and change only the type, or rename it to
`provenance_ref`.

This draft recommends `provenance_ref` because it matches ADR-0104's planned
mutation shape and makes reference semantics explicit.

The runtime retention policy for invocation-addressable provenance should be
confirmed.

If WordPress Studio artifacts can outlive local runtime provenance retention,
the surface needs either longer retention for published composition
invocations or a durable exported provenance summary.

That exported summary should still not be a full envelope per block.

It should be a single artifact-level provenance bundle plus block refs into
that bundle.

The exact JSON Pointer type name should be aligned with existing code.

ADR-0105 uses `FieldPath(String)` and describes it as JSON-Pointer-like.

ADR-0130 can either reuse `FieldPath` or introduce `JsonPointer` as a clearer
surface-independent alias.

If `JsonPointer` is introduced, it should be a newtype with validation rather
than an unconstrained string.

The resolver API should define whether unauthorized provenance returns
`NotFound`, `Unauthorized`, or `Masked`.

For privacy, external callers may need a deliberately ambiguous response.

First-party app diagnostics can use a more precise internal error.

The amendment should confirm whether every block requires a provenance ref.

Some blocks may be purely presentational: spacer, divider, static CTA, or
surface chrome.

Those blocks should either be outside the semantic `Block` list or carry an
explicit `ProvenanceOptional` reason.

They should not use an empty or fake ref.

The size guard threshold of 48KB for block refs is proposed, not yet measured.

It should be validated against representative WordPress Studio documents during
the DOS-546 spike.

If typical documents approach that threshold, the schema should support
section-level refs for repeated child blocks derived from the same field.

The cache invalidation marker for privacy masking is not yet specified.

If provenance can be masked after a client has cached it, the runtime should
provide either a `provenance_revision`, `masked_at`, or source-revocation epoch
so surface clients can avoid rendering stale source detail.
