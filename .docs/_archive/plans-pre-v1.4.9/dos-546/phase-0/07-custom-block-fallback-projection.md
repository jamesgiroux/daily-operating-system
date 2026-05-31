---
status: spec:ready
date: 2026-05-10
amends_adr: "0130"
related_adrs: [0102, 0105, 0108]
open_questions: see ./INDEX.md (routed to W0-A, W4-E, W1-F L0 Prep)
---

# Custom Block Fallback Projection Rules - ADR-0130 Amendment Draft

## Context

ADR-0130 defines the surface-independent composition contract for rendering DailyOS
blocks across product surfaces.

The current ADR-0130 section 3 fallback language allows a renderer that encounters
an unknown block type to fall back to a generic "raw payload available" treatment.

The DOS-546 L0 CSO review flagged that behavior as an information-disclosure vector.

The leak is structural, not cosmetic:

- The renderer does not know the unknown block type's intended display contract.
- The renderer does not know whether payload fields are safe for the current surface.
- The renderer may receive fields whose claim sensitivity is `Internal`,
  `Confidential`, or `UserOnly`.
- The renderer may receive fields that were never intended to be shown at all.
- A generic raw-payload fallback bypasses schema review and product vocabulary.
- Raw payload display can expose internal identifiers, source fragments, emails,
  notes, prompts, or debug carrier fields.

ADR-0102 requires structured, synthesized product output to pass through abilities
and to carry provenance via `AbilityOutput<T>`.

ADR-0105 makes provenance a first-class envelope and places field attribution in
that envelope rather than in each domain payload.

ADR-0108 requires actor-filtered provenance rendering, sanitizer rules for
untrusted explanatory text, and a hard 64 KB serialized provenance cap.

ADR-0125, though not listed in the frontmatter, supplies the current sensitivity
tier vocabulary consumed by render policy:

- `Public`
- `Internal`
- `Confidential`
- `UserOnly`

The corrected ADR-0130 fallback must treat block payload as untrusted display input
unless the payload field is explicitly admitted by a known block schema.

This amendment therefore changes fallback from "raw payload available" to
"schema-bounded projection onto the nearest known block type."

The goal is not to make unknown blocks look perfect.

The goal is to preserve user-visible continuity while preventing unknown fields
from reaching the renderer.

Source note for this draft: `.docs/decisions/0130-surface-independent-composition-contract.md`
and `.docs/reviews/dos-546-l0-cso-2026-05-10.md` were not present in this
worktree under the requested paths when this artifact was produced. This draft
uses the DOS-546 prompt's ADR-0130 and CSO-review excerpts as the primary source
for those two missing inputs, and the repository's ADR-0102, ADR-0105,
ADR-0108, and ADR-0125 files for the surrounding contract.

## Threat Model

The current fallback leaks because it treats "unknown type" as a reason to show
more data, when it should be a reason to show less.

An attacker or buggy producer can create a block whose type is unknown to the
WordPress renderer but whose payload contains sensitive fields.

Examples:

- `/sensitive_email`
- `/private_note`
- `/source_excerpt`
- `/prompt_context`
- `/internal_entity_id`
- `/debug_trace`
- `/raw_claim_text`

If ADR-0130 permits "raw payload available", the renderer may display those fields
or make them reachable through DOM attributes, serialized props, block comments,
debug panels, inspector UI, REST preload state, or client-side hydration data.

The user does not need to click an explicit "debug" affordance for this to be a
leak. If the payload reaches the rendering surface, browser extensions, scripts,
plugins, copied HTML, published pages, cached previews, or logs may capture it.

Unknown block fallback must therefore obey the same principle as ADR-0108:
render through a constrained, actor-appropriate surface contract rather than
dumping internal structure.

Hard rule:

**Never render unknown payload fields directly.**

This hard rule applies even when an unknown payload field has the same name as a
field on a known block.

A field may pass through only when all of the following are true:

- The fallback algorithm selected a deterministic nearest-known block type.
- The field's JSON Pointer appears in the unknown block schema.
- The same JSON Pointer appears in the selected nearest-known block schema.
- The field shape is compatible under the projection rules below.
- The selected nearest-known block schema allows the field for the target surface.
- The field survives the applicable sensitivity ceiling and render policy.

Fields that fail any condition are dropped.

The fallback path must preserve `claim_refs` and `provenance_ref`, but those are
not payload fields.

They are references into the claim and provenance substrates.

Preserving them allows downstream link resolution, provenance affordances, and
"about this" experiences to keep working without exposing the unknown payload.

## Algorithm Spec

### Inputs

The fallback algorithm receives a single unknown block value:

```ts
type UnknownBlock = {
  type: string;
  payload: JsonValue;
  claim_refs: ClaimRef[];
  provenance_ref: ProvenanceRef | null;
};
```

The algorithm also receives registry context:

```ts
type BlockDescriptor = {
  type: string;
  composition_kind: CompositionKind;
  schema: JsonSchema;
  required_pointers: JsonPointer[];
  render_annotations: RenderAnnotation[];
  allowed_surfaces: Surface[];
  default_trust_band?: TrustBand;
};

type FallbackContext = {
  surface: Surface;
  actor: Actor;
  known_blocks: BlockDescriptor[];
  schema_for_unknown_type?: JsonSchema;
};
```

The unknown block schema should come from the composition manifest, block registry
metadata, or a versioned schema bundle shipped with the composition artifact.

If no unknown-type schema is available, the algorithm must treat the unknown
schema as empty.

It must not infer display-allowed pointers from the raw payload.

Inferring schema from payload would reintroduce the leak by allowing an attacker
to choose field names that look safe.

### Pseudocode

```ts
function render_unknown_block_fallback(
  block: UnknownBlock,
  ctx: FallbackContext
): RenderedBlock {
  const unknown_schema =
    ctx.schema_for_unknown_type ?? empty_json_schema();

  const nearest =
    select_nearest_known_type(block.type, unknown_schema, ctx);

  if (nearest == null) {
    return render_generic_text_fallback(block, ctx);
  }

  const unknown_pointers =
    schema_leaf_pointers(unknown_schema);

  const nearest_pointers =
    schema_leaf_pointers(nearest.schema)
      .filter((ptr) => schema_allows_surface(nearest.schema, ptr, ctx.surface))
      .filter((ptr) => schema_allows_actor(nearest.schema, ptr, ctx.actor));

  const intersected_pointers =
    sorted_json_pointer_intersection(unknown_pointers, nearest_pointers)
      .filter((ptr) =>
        schema_shapes_compatible(unknown_schema, nearest.schema, ptr)
      );

  const projected_payload = {};

  for (const ptr of intersected_pointers) {
    if (json_pointer_exists(block.payload, ptr)) {
      set_json_pointer(
        projected_payload,
        ptr,
        get_json_pointer(block.payload, ptr)
      );
    }
  }

  const rendered = render_known_block({
    type: nearest.type,
    payload: projected_payload,
    claim_refs: block.claim_refs,
    provenance_ref: block.provenance_ref,
    trust_band: degrade_trust_band(
      nearest.default_trust_band,
      "needs_verification"
    ),
    fallback_banner: {
      visible: true,
      dismissible: false,
      text: `Rendered as ${nearest.type} — payload may be incomplete`,
    },
    fallback_metadata: {
      original_type: block.type,
      projected_pointer_count: intersected_pointers.length,
      dropped_unknown_payload: true,
    },
  }, ctx);

  return rendered;
}
```

### Generic Fallback Pseudocode

```ts
function render_generic_text_fallback(
  block: UnknownBlock,
  ctx: FallbackContext
): RenderedBlock {
  return render_known_block({
    type: "dailyos/text",
    payload: {},
    claim_refs: block.claim_refs,
    provenance_ref: block.provenance_ref,
    trust_band: "needs_verification",
    fallback_banner: {
      visible: true,
      dismissible: false,
      text: "Rendered as dailyos/text — payload may be incomplete",
    },
    fallback_metadata: {
      original_type: block.type,
      projected_pointer_count: 0,
      dropped_unknown_payload: true,
    },
  }, ctx);
}
```

### Projection Rules

Projection happens at JSON Pointer granularity.

The algorithm computes the intersection of leaf pointers between the unknown
block schema and the selected nearest-known block schema.

A pointer is eligible only if the same pointer exists in both schemas.

The renderer must not pass through sibling objects wholesale.

For example, if `/summary/text` intersects but `/summary/private_note` does not,
only `/summary/text` is copied.

The object at `/summary` is reconstructed as a container because it is needed to
hold the allowed child pointer.

It is not copied as a whole object from the unknown payload.

Array projection is allowed only for array item schemas with matching pointer
patterns.

For example, `/items/*/title` may project when both schemas declare an array at
`/items` and both item schemas declare `/title` with compatible type.

Array projection does not allow unknown item fields through.

For each item, the renderer reconstructs a new item with only intersected item
pointers.

Scalar projection requires compatible scalar types.

Compatible scalar types are exact type matches, or a known safe widening declared
by the nearest-known schema.

Safe widening examples:

- integer to number
- enum member to string only when the nearest-known schema marks the field as
  display text

Unsafe widening examples:

- object to string
- array to string
- arbitrary JSON value to string
- number to date string

No field may be rendered by calling `JSON.stringify` on an unknown value.

No field may be rendered by dumping a pretty-printed unknown object.

No field may be rendered through a catch-all key/value table.

No unknown payload field may be placed into `data-*` attributes, hidden inputs,
HTML comments, script tags, serialized block attributes, inspector panels, REST
preload state, or debug panels.

The fallback path must preserve `claim_refs` exactly.

`claim_refs` are not part of `payload`.

They are stable references into the claim substrate and are required for
downstream link resolution.

The fallback path must preserve `provenance_ref` exactly.

`provenance_ref` is not part of `payload`.

It is a reference into the provenance envelope governed by ADR-0105 and rendered
through ADR-0108 actor-filtered rules.

The fallback path must not dereference `claim_refs` or `provenance_ref` to fill
missing payload fields.

Dereferencing for the "About this" or source-detail affordance is allowed only
through the normal renderer paths and sensitivity gates.

### Failure Semantics

Fallback rendering is a soft degradation, not a hard error.

The block still renders, but with a visible banner and a degraded trust band.

The renderer should emit a structured diagnostic for operator visibility:

```ts
type UnknownBlockFallbackDiagnostic = {
  original_type: string;
  selected_type: string;
  surface: Surface;
  projected_pointer_count: number;
  dropped_payload_pointer_count?: number;
  reason: "unknown_block_type";
};
```

The diagnostic must not contain dropped payload values.

The diagnostic may contain pointer names if pointer names are already present in
schemas and are not sensitive by policy.

If pointer names themselves are considered sensitive for a surface, the diagnostic
should contain counts only.

## Nearest-Known-Type Selection

Nearest-known type selection must be deterministic.

It must not depend on runtime payload values.

It may depend on schemas, declared block metadata, and type identifiers.

Payload-value-based matching is forbidden because sensitive fields should not be
read for type selection.

Recommended selection algorithm:

```ts
function select_nearest_known_type(
  unknown_type: string,
  unknown_schema: JsonSchema,
  ctx: FallbackContext
): BlockDescriptor | null {
  const candidates = ctx.known_blocks
    .filter((d) => d.allowed_surfaces.includes(ctx.surface))
    .filter((d) => can_render_for_actor(d, ctx.actor));

  if (candidates.length === 0) {
    return null;
  }

  const unknown_kind =
    declared_composition_kind(unknown_type) ??
    composition_kind_from_schema_annotations(unknown_schema) ??
    null;

  const scored = candidates.map((candidate) => ({
    candidate,
    score: nearest_type_score(unknown_type, unknown_schema, unknown_kind, candidate),
  }));

  const viable = scored.filter((x) => x.score.required_overlap > 0);

  const pool = viable.length > 0 ? viable : scored;

  pool.sort(compare_nearest_type_scores);

  const winner = pool[0];

  if (winner.score.total <= 0) {
    return null;
  }

  return winner.candidate;
}
```

Recommended score components:

```ts
type NearestTypeScore = {
  kind_match: 0 | 100;
  required_overlap: number;
  optional_overlap: number;
  annotation_similarity: number;
  type_namespace_similarity: number;
  total: number;
};
```

Score each component as follows:

- `kind_match`: 100 when `composition_kind` matches.
- `required_overlap`: 10 points for each compatible required pointer shared by
  both schemas.
- `optional_overlap`: 2 points for each compatible optional pointer shared by
  both schemas.
- `annotation_similarity`: 0 to 20 points for matching render annotations such
  as list, quote, metric, timeline, callout, person, account, meeting, or source.
- `type_namespace_similarity`: 0 to 5 points for matching namespace prefixes,
  such as `dailyos/meeting-*` to `dailyos/meeting-summary`.

Tie-break rule:

1. Higher `total` wins.
2. Higher `kind_match` wins.
3. Higher `required_overlap` wins.
4. Higher `optional_overlap` wins.
5. Higher `annotation_similarity` wins.
6. Lexicographically smaller `type` wins.

The final tie-break is alphabetical type id so the result is deterministic across
machines, runtimes, and registry iteration order.

When no candidate has positive score, render as generic `dailyos/text`.

When candidates exist but the selected candidate has no schema intersection with
the unknown schema, render as generic `dailyos/text`.

The generic fallback uses an empty payload.

It preserves `claim_refs`.

It preserves `provenance_ref`.

It shows the fallback banner.

It sets trust band to `needs_verification`.

### Required Field Overlap

Required-field overlap is the strongest predictor after `composition_kind`.

If an unknown block declares required pointers `/title` and `/body`, a known text
or callout block with the same required pointers is a safer fallback than a metric
block with only `/value`.

However, required overlap only permits selection.

It does not permit raw payload rendering.

The projection step still controls exactly which fields flow into the rendered
payload.

### Annotation Similarity

Annotation similarity is useful when two blocks share a shape but differ in
product intent.

For example, a meeting "topics" block and a generic "bulleted list" block may
both expose `/items/*/title`, but the meeting-specific block may have annotations
for `meeting`, `agenda`, or `decision`.

The selector should prefer a matching product intent when available.

Annotations must be curated metadata.

They must not be free-text embeddings computed from payload values.

### No Schema Intersection

If the unknown schema has no eligible pointer intersection with the selected
known schema, the renderer must not create display text from the payload.

It must render the generic `dailyos/text` fallback with an empty payload.

This rule intentionally produces a sparse block.

Sparse-but-safe is the correct behavior for unknown content.

## Banner Rules

The WordPress renderer must show the fallback banner visually on the block.

It should be inside the block boundary, above the projected content, and styled as
a quiet warning state rather than as a destructive error.

Exact banner string for projected fallback:

> Rendered as `<nearest-known-type>` — payload may be incomplete

Exact banner string for generic fallback:

> Rendered as `dailyos/text` — payload may be incomplete

The banner copy intentionally uses product-facing vocabulary.

It does not use internal vocabulary such as:

- enrichment
- intelligence pipeline
- substrate
- LLM
- schema projection
- unknown type

Those concepts belong in logs, diagnostics, and developer documentation, not in
the content block seen by a WordPress editor or reader.

The banner should include the selected known type only because the type is useful
for support and QA.

If product review decides that type ids are too internal for WordPress users, the
copy may map the type id through a display label:

> Rendered as Text — payload may be incomplete

For the ADR amendment, keep the canonical machine-facing string with type id so
test fixtures can assert it deterministically.

Trust band rule:

- Every fallback block degrades to `needs_verification`.
- If the original block had a stronger trust band, fallback caps it at
  `needs_verification`.
- If the original block was already `needs_verification`, it remains there.
- Fallback must not upgrade trust.

The trust-band degradation reflects uncertainty in presentation, not necessarily
uncertainty in the underlying claims.

The preserved `claim_refs` may still resolve to stronger or weaker claim-level
trust when the user opens details.

The visible block itself should communicate that its rendered shape is degraded.

Dismissibility recommendation:

- The banner should be non-dismissible in Phase 1.
- A fallback block is a persistent rendering caveat, not a transient notice.
- Dismissing it would make the block look first-class while it is still
  schema-degraded.
- Non-dismissible display also helps QA and support identify fallback projection
  during the viability spike.

If later product work adds dismissal, dismissal must be per viewer/session and
must not remove the diagnostic from exported or published surfaces unless a
separate ADR defines that policy.

The banner must render even when the projected payload is empty.

The banner must render even when the block has valid `claim_refs` and
`provenance_ref`.

The banner must not include dropped field names or dropped field values.

The banner must not include sensitivity labels.

The banner must not expose the original unknown type unless product review
explicitly approves that for the surface.

## Phase 1 Acceptance Fixtures

### Fixture 1: Unknown Block With Sensitive Email Field

Purpose:

Verify that fallback projection does not leak a sensitive payload field.

Input:

```json
{
  "type": "dailyos/custom-stakeholder-note-v2",
  "payload": {
    "title": "Renewal risk",
    "body": "Customer wants legal review before renewal.",
    "sensitive_email": "alex@example.com",
    "private_note": "Do not share this outside the account team."
  },
  "claim_refs": ["claim_renewal_risk_001"],
  "provenance_ref": "prov_prepare_meeting_001"
}
```

Unknown schema:

```json
{
  "type": "object",
  "required": ["title", "body"],
  "properties": {
    "title": { "type": "string" },
    "body": { "type": "string" },
    "sensitive_email": { "type": "string", "sensitivity": "UserOnly" },
    "private_note": { "type": "string", "sensitivity": "Confidential" }
  }
}
```

Nearest-known type:

`dailyos/callout`

Known schema:

```json
{
  "type": "object",
  "required": ["title", "body"],
  "properties": {
    "title": { "type": "string" },
    "body": { "type": "string" },
    "tone": { "type": "string", "enum": ["info", "warning", "success"] }
  }
}
```

Expected projected payload:

```json
{
  "title": "Renewal risk",
  "body": "Customer wants legal review before renewal."
}
```

Verification:

- Rendered output contains `title`.
- Rendered output contains `body`.
- Rendered output does not contain `sensitive_email`.
- Rendered output does not contain `alex@example.com`.
- Rendered output does not contain `private_note`.
- Rendered output does not contain `Do not share this outside the account team`.
- Serialized block props do not contain dropped fields.
- DOM does not contain dropped fields.
- HTML comments do not contain dropped fields.
- REST preload or hydration state does not contain dropped fields.
- Banner is visible.
- Trust band is `needs_verification`.

### Fixture 2: Claim Refs and Provenance Ref Preserved Across Fallback

Purpose:

Verify that fallback projection preserves substrate links even while dropping
unsafe payload fields.

Input:

```json
{
  "type": "dailyos/custom-meeting-topic-v3",
  "payload": {
    "heading": "Pricing follow-up",
    "summary": "Confirm enterprise plan packaging.",
    "source_excerpt": "Long raw quoted text omitted from fallback."
  },
  "claim_refs": [
    "claim_pricing_followup_001",
    "claim_enterprise_packaging_002"
  ],
  "provenance_ref": "prov_prepare_meeting_002"
}
```

Nearest-known type:

`dailyos/topic-summary`

Expected behavior:

- Project only intersected payload pointers.
- Drop `/source_excerpt` unless `dailyos/topic-summary` explicitly declares it.
- Preserve both claim refs exactly.
- Preserve `provenance_ref` exactly.
- Render downstream claim-link affordances using preserved refs.
- Render "About this" or provenance affordance through ADR-0108 rules.
- Do not copy provenance content into payload.
- Do not dereference claims to backfill dropped payload fields.

Verification:

- `rendered.claim_refs` equals the input `claim_refs`.
- `rendered.provenance_ref` equals the input `provenance_ref`.
- Link resolution can open both claim records.
- Provenance detail can be requested from `prov_prepare_meeting_002`.
- Rendered block body does not include `source_excerpt`.
- Banner is visible.
- Trust band is `needs_verification`.

### Fixture 3: Unknown Block With No Schema Intersection

Purpose:

Verify that fallback does not invent display text when schemas do not intersect.

Input:

```json
{
  "type": "dailyos/custom-risk-graph-v1",
  "payload": {
    "nodes": [
      { "id": "n1", "label": "Legal review" }
    ],
    "edges": [
      { "from": "n1", "to": "n2", "reason": "Dependency" }
    ],
    "sensitive_email": "legal@example.com"
  },
  "claim_refs": ["claim_legal_dependency_001"],
  "provenance_ref": "prov_detect_risk_shift_001"
}
```

Unknown schema:

```json
{
  "type": "object",
  "required": ["nodes", "edges"],
  "properties": {
    "nodes": { "type": "array" },
    "edges": { "type": "array" },
    "sensitive_email": { "type": "string", "sensitivity": "UserOnly" }
  }
}
```

Available known schemas:

- `dailyos/text`, with optional `/text`
- `dailyos/callout`, with required `/title` and `/body`
- `dailyos/metric`, with required `/label` and `/value`

Expected behavior:

- No eligible pointer intersection is found.
- Renderer falls back to generic `dailyos/text`.
- Payload is entirely dropped.
- `claim_refs` is preserved.
- `provenance_ref` is preserved.
- Banner is visible.
- Trust band is `needs_verification`.

Verification:

- Rendered type is `dailyos/text`.
- Rendered payload is `{}`.
- Rendered output does not contain `nodes`.
- Rendered output does not contain `edges`.
- Rendered output does not contain `Legal review`.
- Rendered output does not contain `Dependency`.
- Rendered output does not contain `sensitive_email`.
- Rendered output does not contain `legal@example.com`.
- Downstream link resolution can still open `claim_legal_dependency_001`.
- Provenance detail can still be requested from `prov_detect_risk_shift_001`.

## Drop-In Amendment Text for ADR-0130 §3

```markdown
### 3.x Unknown Block Fallback Projection

Renderers MUST NOT render unknown block payload fields directly.

The previous "raw payload available" fallback is removed. An unknown block type is
a privacy boundary, not a debugging opportunity. Unknown payload fields may include
internal notes, source excerpts, email addresses, prompt context, debug carriers, or
fields whose sensitivity tier is not allowed on the current surface.

When a renderer encounters a block whose `type` is not known by that renderer, it
MUST use schema-bounded fallback projection:

1. Identify the nearest known block type using deterministic registry metadata:
   first prefer matching `composition_kind`, then higher compatible required-field
   overlap, then higher compatible optional-field overlap, then render annotation
   similarity, then namespace similarity. Ties MUST be broken by lexicographic
   block type id so all renderers choose the same fallback.
2. Compute the JSON Pointer intersection between the unknown block's declared
   schema and the selected nearest-known block type's schema.
3. Project ONLY those intersected pointers from the unknown payload into a new
   payload for the nearest-known block type.
4. Drop every non-intersected payload field. This includes fields with identical
   names that are not present in the selected nearest-known block type's schema.
5. Always preserve `claim_refs`. These are not payload; they are references into
   the claim substrate and are required for downstream linking.
6. Always preserve `provenance_ref`. This is not payload; it is a reference into
   the ADR-0105 provenance envelope and renders only through ADR-0108 actor-filtered
   provenance rules.
7. Render the block as the selected nearest-known block type using the projected
   payload, preserved `claim_refs`, and preserved `provenance_ref`.
8. Show a visible, non-dismissible banner on the block:
   `Rendered as <nearest-known-type> — payload may be incomplete`.
9. Cap the rendered block trust band at `needs_verification`.

If no nearest-known type has a positive deterministic match, or if the selected
nearest-known type has no eligible schema intersection with the unknown schema, the
renderer MUST render the block as generic `dailyos/text` with an empty payload.
The generic fallback still preserves `claim_refs` and `provenance_ref`, still shows
the banner `Rendered as dailyos/text — payload may be incomplete`, and still caps
the trust band at `needs_verification`.

Renderers MUST NOT infer display-allowed fields from the raw payload. If the
unknown block's declared schema is unavailable, the unknown schema is treated as
empty and the generic `dailyos/text` fallback is used.

Renderers MUST NOT place dropped payload fields into visible text, hidden DOM,
`data-*` attributes, HTML comments, serialized block attributes, REST preload
state, hydration state, inspector UI, debug panels, logs, or diagnostics.
Diagnostics may include counts and non-sensitive type ids; they MUST NOT include
dropped payload values.

The fallback path is a soft degradation. It preserves substrate references for
claim linking and provenance affordances while refusing to expose unknown payload
content outside a known schema contract.
```

## Open Questions

1. Where will ADR-0130 live once it is added to this worktree?

The requested path `.docs/decisions/0130-surface-independent-composition-contract.md`
was absent while this draft was written. The amendment above is phrased as a
drop-in subsection, but final placement should be checked against the actual
ADR-0130 section 3 structure.

2. Where should the unknown block schema be sourced from?

Preferred order is composition manifest, block registry metadata, then versioned
schema bundle. If none exists, the generic empty-payload fallback applies.

3. Should the WordPress renderer expose type ids in banners?

The deterministic test string uses type ids. Product copy may prefer display
labels, but tests need a stable canonical source.

4. Should fallback diagnostics include dropped pointer names?

Counts are always safe. Pointer names are probably acceptable in developer-only
diagnostics, but product surfaces should avoid them unless a surface policy says
they are non-sensitive.

5. Should `composition_kind` be mandatory for all block descriptors?

This draft recommends using it as the first selector. Making it mandatory would
improve fallback quality and reduce reliance on schema overlap.

6. Should fallback blocks prevent publication?

This draft only degrades trust band and shows a banner. A later publish policy
may choose to block external publication when any block is fallback-rendered.

7. How should array length be handled during projection?

This draft preserves array length while dropping non-intersected item fields. A
future implementation may cap projected array length per block schema to avoid
overlarge fallback renderings.

8. Should schema annotations carry per-surface sensitivity ceilings?

Claim sensitivity is substrate-level, but block payload schema may need its own
display ceilings for non-claim payload fields.

9. Should the generic `dailyos/text` fallback display a placeholder line?

This draft recommends empty payload plus banner. A placeholder like "Content could
not be shown in this view" may be useful, but it should be product-reviewed and
must not be derived from payload.

10. Should fallback projection be implemented in a shared package?

Yes, if multiple surfaces consume ADR-0130 blocks. The selection, projection, and
diagnostic rules should be shared so WordPress, Tauri, MCP, and future renderers
do not drift.
