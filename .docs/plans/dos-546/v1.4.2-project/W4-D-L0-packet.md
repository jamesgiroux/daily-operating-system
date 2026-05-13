# W4-D L0 packet — substrate fallback projection + edit-routing rules

Date: 2026-05-13 (V3)
Project: v1.4.2 — Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: 4 stage-2 (gates W4-A renderer and W5-A feedback routing)
Issue: DOS-570 (W4-D: substrate-side fallback projection + edit-routing rules)
Depends on: DOS-556 (W1-E Composition types) + DOS-567 (W4-B V8 BindingRole contract)

This packet captures the W4-D contract decisions resolved at L0. The Linear
issue description remains the canonical execution contract; this packet
supersedes it only where it makes explicit a decision the issue leaves open.

## Changelog

- **V3 (2026-05-13):** Cycle 2 conditional reviewer fold. Resolves codex
  N1 by keeping the projector pure inside `abilities-runtime`: the public API
  returns `(ProjectedComposition, Vec<AuditIntent>)`, and the app/service caller
  drains those intents through `emit_surface_audit(...)`. Resolves codex N2 by
  pinning `ProjectionDiagnostic` to the §9 field list, including
  `composition_id`, `composition_version`, `original_type_id`, and `reason`.
  Folds eng P3-new-1/P3-new-2 by closing `ProducerOutputInvalidReason` and
  `EditRouteRefusalReason` as enums. Folds devex P3-DX-1 by expanding the
  public re-export list, and removes the audit error variant because audit
  failure is caller-owned after N1.
- **V2 (2026-05-13):** Cycle 1 reviewer fold. Promotes
  `project_composition_for_surface(...) -> ProjectedComposition` as the public
  API, with block helpers internal and cap enforcement inside the composition
  projector per W4-B V8. Adds DTO/module/registry pins, fail-closed
  `unknown_role`, schema-field classification with `fallback_policy_version`,
  exact audit detail schemas, SurfaceClient scope projection, hardened enum
  widening, ComputedFrom/DisplayOnly scope gates, FeedbackTarget with 0 claim
  refs behavior, and acceptance section 53 plus section 54 for sequencing and
  asymmetric forward-compat. Notes W4-B §17/§37 endpoint inheritances have no
  direct W4-D endpoint work.
- **V1 (2026-05-13):** Initial L0 packet. Folds Linear DOS-570, W4-B V7
  surfaces (`Block.field_bindings`, `BindingRole`, `ClaimRef.field_path`),
  ADR-0130 §3 as amended by phase-0 artifact 07, and artifact 11 edit-routing
  semantics. Resolves banner-copy precedence in favor of Linear's newer
  product-facing copy: "Rendered as nearest known type — payload may be
  incomplete." Selected type id remains available only in diagnostics/audit.

## Status snapshot

- W1-E landed the base `Composition`, `Block`, `ClaimRef`, `ProvenanceRef`, and
  block fallback skeleton. W4-D consumes that shape; it does not re-author it.
- W4-B V8 is the hard dependency. W4-D implementation starts after W4-B merges
  or the branch is rebased onto W4-B's `FieldBinding` / `BindingRole` surfaces.
- W4-D is pure substrate policy. It requires **no migration** and claims no
  migration slot.
- W4-A renderer may not infer fallback policy locally. It consumes W4-D's
  substrate-published `ProjectedComposition` and edit-routing metadata.
- W5-A feedback router may not guess from Gutenberg diffs. It consumes W4-D's
  role-dispatched edit routes and refuses fields with no explicit receiver.
- Linear DOS-570 acceptance is newer than ADR-0130's exact banner example. This
  packet treats Linear's copy as canonical for v1.4.2.
- W4-B V8 inherited interlock: composition-level projection is the only public
  surface; block-level fallback helpers are private implementation details.
- W4-B §17 `wp_user_id` session binding and §37 `surface_client.rs` endpoint
  ownership are acknowledged inherited constraints. W4-D has no endpoints, so
  it has no direct bridge route or session-binding implementation obligation.

## Pre-work confirmed (substrate reuse audit)

**Headline finding:** W4-D is not a greenfield renderer feature. The repository
already contains a partial fallback projector in the composition substrate, but
it lacks the W4-B role topology, sensitivity-aware admitted-field registry,
audit/cap contract, and W5-A edit-routing metadata that DOS-570 requires.

### Already in `src-tauri/abilities-runtime/src/abilities/composition.rs`

- **`BlockType` taxonomy** exists with canonical variants:
  `AccountOverview`, `ClaimSummary`, `EvidenceList`, `HealthSnapshot`,
  `RelationshipMap`, `RiskCallout`, `ActionList`, `MarkdownDocument`, and
  `Custom { type_id }`.
- **`ClaimRef { claim_id, claim_version }`** exists and is preserved by the
  fallback skeleton. W4-B adds `field_path`; W4-D must consume that field for
  pointer-level attribution and edit routing.
- **`ProvenanceRef { invocation_id, field_path }`** exists and validates
  against the canonical provenance envelope. W4-D preserves it exactly; it does
  not dereference it to reconstruct dropped fields.
- **`Block { attributes, claim_refs, provenance }`** exists. W4-B adds
  `field_bindings: Vec<FieldBinding>`; W4-D treats those bindings as the
  routing authority.
- **`project_to_nearest_known(...)`** exists with deterministic nearest-type
  scoring and pointer reconstruction. W4-D must harden it into a published
  policy surface: declared schemas only, sensitivity-aware admitted fields,
  raw-payload exclusion across DOM/JS/logs, audit-intent generation, caller
  emission, and cap handling.
- **Current gap:** existing fallback banner includes the selected type id
  (`Rendered as {selected_type_id} — payload may be incomplete`). DOS-570 pins
  generic user-visible copy. W4-D must update the product-facing banner while
  keeping selected type id in diagnostics.

### Already in `src-tauri/abilities-runtime/src/sensitivity.rs`

- **Render policy by sensitivity and surface** exists for `Public`, `Internal`,
  `Confidential`, and `UserOnly`.
- W4-D should reuse this policy path when deciding whether an admitted field is
  renderable, redacted, or dropped for a `SurfaceClient`.
- W4-D must not introduce a parallel sensitivity check or stringly typed copy of
  the sensitivity ladder.

### Already in W4-B V8 contract

- **`Block.field_bindings: Vec<FieldBinding>`** is the substrate-published field
  topology for a block.
- **`BindingRole` enum** is:
  `Source`, `ComputedFrom`, `DisplayOnly`, `FeedbackTarget`.
- **`ClaimRef.field_path`** is optional on the claim ref but mandatory for
  `Source` and `FeedbackTarget` bindings that address claim fields.
- **`project_composition_for_surface(...)` interlock** is inherited from W4-B
  V8: W4-D publishes a composition-level surface, enforces the unknown-block cap
  inside that surface, and keeps block helpers internal.
- **Class-level scope-filter rule from W4-B §16** applies to W4-D projection:
  rendered content for `Actor::SurfaceClient` must be filtered through the same
  requester scopes before it reaches W4-A, MCP, CLI, or Tauri surfaces.
- W4-D is the primary first consumer of this V8 surface. If W4-B changes the
  role enum or field names, W4-D packet V3 or its successor must fold that
  before implementation.

### Already in `src-tauri/src/audit_log.rs`

- **`emit_surface_audit(...)`** exists and enforces `SurfaceClient` audit shape.
- W4-D audit writes must go through service-layer code that calls the existing
  audit helper. No command handler writes audit rows directly.

## Directional decisions resolved at L0

### §1. W4-D publishes policy; renderers consume projections

W4-D's output is a substrate policy surface, not a WordPress-only helper.

The implementation exposes one public Rust API that all surfaces call:

```rust
pub fn project_composition_for_surface(
    composition: &Composition,
    ctx: &FallbackProjectionContext,
) -> Result<(ProjectedComposition, Vec<AuditIntent>), ProjectionError>;
```

The exact module name is implementation-owned, but the boundary is not:

- input is a substrate `Composition` plus actor/surface context;
- output is a projected composition DTO safe for a surface to render;
- output carries per-block banner, trust-band cap, diagnostics, and edit routes;
- output carries composition-level cap diagnostics and audit intents;
- output never carries dropped raw values;
- audit emission itself is not in `abilities-runtime`; callers in the app crate
  emit returned intents through existing audit helpers.

Block-level helpers such as `project_block_for_surface_internal(...)` may exist,
but they are `pub(crate)` or private implementation details. They may not be the
normative API, because the unknown-block cap is composition-level state. W4-A may
adapt `ProjectedComposition` into Gutenberg render props. It may not call block
helpers directly, recompute nearest type, count unknown blocks, enforce the cap,
re-run sensitivity admissibility, or rebuild edit routing.

### §1.1. DTO and module shape pinned for implementation

W4-D lands in
`src-tauri/abilities-runtime/src/abilities/fallback_projection.rs` unless the
implementation proves a smaller diff by extending
`src-tauri/abilities-runtime/src/abilities/composition.rs`. Either placement must
re-export the public API through the existing abilities module path so consumers
can import:

```rust
pub use abilities_runtime::abilities::{
    project_composition_for_surface,
    AuditIntent,
    EditRoute,
    FallbackProjectionContext,
    ProjectionDiagnostic,
    ProjectionError,
    ProjectedComposition,
};
```

The DTO shape is pinned at L0 so W4-A/W5-A do not invent parallel wire shapes:

```rust
pub struct ProjectedComposition {
    pub composition_id: CompositionDocId,
    pub composition_version: Option<u64>,
    pub fallback_policy_version: u32,
    pub blocks: Vec<ProjectedBlock>,
    pub diagnostics: Vec<ProjectionDiagnostic>,
    pub unknown_block_count: u32,
    pub unknown_block_cap: u32,
    pub dropped_unknown_block_count: u32,
}

pub struct ProjectedBlock {
    pub block_id: BlockId,
    pub block_index: u32,
    pub original_type_id: String,
    pub selected_known_type_id: String,
    pub payload: serde_json::Value,
    pub banner: Option<String>,
    pub trust_band: TrustBand,
    pub claim_refs: Vec<ClaimRef>,
    pub provenance: Vec<ProvenanceRef>,
    pub edit_routes: Vec<EditRoute>,
    pub diagnostics: Vec<ProjectionDiagnostic>,
}

pub struct EditRoute {
    pub field_path: FieldPath,
    pub role: BindingRole,
    pub claim_refs: Vec<ClaimRef>,
    pub feedback_allowed: bool,
    pub refusal_reason: Option<EditRouteRefusalReason>,
}

pub struct ProjectionDiagnostic {
    pub diagnostic_kind: DiagnosticKind,
    pub composition_id: CompositionId,
    pub composition_version: u64,
    pub original_type_id: Option<String>,
    pub selected_known_type_id: Option<String>,
    pub block_id: Option<BlockId>,
    pub reason: DiagnosticReason,
    pub dropped_pointer_count: u32,
}

pub struct AuditIntent {
    pub event_kind: &'static str,
    pub category: AuditCategory,
    pub detail: serde_json::Value,
}

#[serde(rename_all = "snake_case")]
pub enum AuditCategory {
    Security,
    DataAccess,
    Ai,
    Anomaly,
    Config,
    System,
}

pub struct FallbackProjectionContext {
    pub actor: Actor,
    pub surface: SurfaceKind,
    pub fallback_policy_version: u32,
    pub unknown_block_cap: u32,
    pub include_non_sensitive_pointer_names: bool,
}

pub enum ProjectionError {
    MissingRule { block_type: BlockType },
    InvalidProducerOutput { reason: ProducerOutputInvalidReason },
}

#[serde(rename_all = "snake_case")]
pub enum ProducerOutputInvalidReason {
    SourceBindingMissingFieldPath,
    FeedbackTargetMissingFieldPath,
    BindingTargetsUnknownField,
    UnknownRole,
    AmbiguousReceiver,
    ConflictingDuplicateBinding,
    MissingDeclaredSchema,
    InvalidFieldPath,
    NonClaimFeedbackReceiverUnsupported,
}

#[serde(rename_all = "snake_case")]
pub enum EditRouteRefusalReason {
    Computed,
    DisplayOnly,
    SourceWithoutTarget,
    MissingClaimRef,
    AmbiguousReceiver,
    SensitivityBlocked,
    UnknownRole,
    OutOfScope,
    FallbackDegradedWithoutReceiver,
}
```

`fallback_policy_version` is a `u32`. It changes when admitted fields, denied
fields, widening rules, fallback banner policy, trust-cap behavior, or
edit-route refusal semantics change. It does not change for runtime payload
content.

`ProjectionError` intentionally has no audit-wrapping variant. Audit
errors are produced by the app/service caller while draining `AuditIntent`
values, not by the pure projector in `abilities-runtime`. The runtime-owned
`AuditCategory` maps one-to-one to the strings accepted by `AuditFields::new`
(`security`, `data_access`, `ai`, `anomaly`, `config`, `system`).

### §2. Migration footprint is none

W4-D requires no table, column, trigger, or migration slot.

All net-new durable behavior is expressed through:

- Rust policy/registry code;
- serialized projection DTOs returned to surfaces;
- audit intents returned by the projector and emitted by callers through the existing audit log;
- tests/fixtures that pin the projection contract.

If implementation discovers a need for persisted registry rows, that is not
DOS-570 as scoped. It requires a successor L0 packet because it would change the migration
footprint and wave slot plan.

### §3. Projection rule registry per known BlockType

Every non-`Custom` `BlockType` must have one substrate `BlockProjectionRule`.
The registry is closed over the enum; a CI/unit test fails when a new known
variant lacks a rule.

Each rule publishes the same information regardless of concrete struct shape:
known field pointers, required/optional pointer sets, sensitivity tier, allowed
surfaces, render annotations, default trust band, and `BindingRole`.

Initial rule coverage required before W4-D can merge:

| BlockType | Admitted field set requirement |
|---|---|
| `AccountOverview` | Account display fields, summary fields, health/risk/action list fields, relationship labels; no raw account JSON catch-all |
| `ClaimSummary` | Claim title/body/status/as-of fields and field-level source/trust metadata |
| `EvidenceList` | Evidence labels/source labels/source-as-of fields; source excerpts only if admitted and sensitivity-gated |
| `HealthSnapshot` | Health band/score/rationale/trend fields |
| `RelationshipMap` | Node labels/roles and edge labels; no hidden identifiers as display fields |
| `RiskCallout` | Title/body/severity/recommended action fields |
| `ActionList` | Item title/status/due-at/owner-label fields |
| `MarkdownDocument` | Title/body/section heading/section body fields; no arbitrary JSON stringify fallback |
| `dailyos/text` generic fallback | Empty payload only for unknown fallback; preserved refs/banner/trust cap |

No rule may contain a wildcard that admits a whole object (`/*`) or an arbitrary
unknown value. Array wildcards are allowed only for declared item schemas such as
`/items/*/title`.

The `BlockProjectionRule` registry lives beside the projector in
`fallback_projection.rs`, or in `composition.rs` only if the implementation keeps
the projector there. The implementation must choose one forward-compatible shape:

- an exhaustive `match BlockType` returning static `BlockProjectionRule` const
  data; or
- a `OnceLock<HashMap<BlockType, BlockProjectionRule>>` with a startup-time
  exhaustiveness assertion over every non-`Custom` variant.

Schema-field classification is mandatory. Every known schema leaf is classified
as `admitted` or `denied`; new leaves on an existing known schema default to
`denied` until explicitly classified. Any admitted-field change bumps
`fallback_policy_version`. Producer payload fields not in the admitted set for
the running substrate version are dropped, audited as `unknown_admitted_field`,
and never raw-rendered.

### §4. BindingRole dispatch matrix

W4-D dispatches on W4-B's V8 `BindingRole`. These roles are additive rows over
field paths; a single field path may have more than one binding row when it both
renders a source and exposes an explicit feedback receiver.

| Role | Meaning | Render behavior | W5-A edit-route behavior |
|---|---|---|---|
| `Source` | 1:1 claim-to-field attribution | Render with provenance and trust band per ADR-0105/0108; preserve `claim_ref`, `claim_version`, and `field_path` | Not sufficient by itself for feedback. If no `FeedbackTarget` row also exists, W5-A refuses correction routing. |
| `ComputedFrom` | N:M derived field computed from one or more claims | Render as computed/derived. Surface may show provenance summary, but the field is not a direct claim value. | W5-A MUST refuse feedback routing. User corrections need an explicit designed flow, not a guessed write to sources. |
| `DisplayOnly` | Surface-local or layout/display field | Read-only render. No provenance/trust requirement unless separately present on a `Source` row. | No feedback target and no edit affordance. Edits stay local or are ignored by substrate routing. |
| `FeedbackTarget` | Explicit feedback receiver for the field | Render may expose correction/dismiss affordance when nonce and surface rules allow. `claim_refs` length may be 0..N, but claim correction requires at least one receiver. | Only role W5-A can route as claim feedback. 0 refs means W5-A refuses claim correction unless a later explicit non-claim receiver exists. |

**No role inference:** W5-A must not infer feedback eligibility from field names,
Gutenberg attributes, or whether a value visually looks like a claim.

**Unknown role fail-closed:** producer-output validation fails when a block
contains an unknown or future `BindingRole`. If projection encounters one anyway
from stale data or deserialization drift, the affected route is refused with
`unknown_role`, a diagnostic is emitted, and `feedback_allowed` is never true.

**Cross-claim scope gate:** `ComputedFrom` render requires requester scope to
permit read access on every claim in `claim_refs`. If any source claim is
out-of-scope, the field is dropped, not blanked and not redacted. `DisplayOnly`
with non-empty `claim_refs` follows the same every-claim gate because its
rendered value can still leak an aggregate signal.

**FeedbackTarget with 0 claim_refs:** the field may render only as read-only
content after normal scope/sensitivity gates. It publishes no claim correction
route, sets `feedback_allowed = false`, uses refusal reason
`missing_claim_ref`, emits a diagnostic, and exposes no nonce-consuming
affordance unless a future explicit non-claim receiver type is added by a later
packet.

### §5. Unknown block fallback algorithm

Unknown `BlockType::Custom { type_id }` degrades by deterministic, schema-bounded
projection onto a known type.

Inputs:

- unknown block value;
- declared schema for the unknown type, sourced from composition manifest,
  block registry, or versioned schema bundle;
- known-type projection registry;
- actor/surface context.

Hard rules:

1. The selector uses declared schemas and metadata only.
2. Runtime payload values are never used for type selection.
3. Missing unknown schema is treated as empty schema.
4. Empty schema or no eligible intersection renders generic `dailyos/text`.
5. Projection happens at JSON Pointer granularity.
6. Container objects are reconstructed only to hold admitted leaves.
7. Sibling objects are never copied wholesale.
8. Array projection rebuilds each item with only intersected item pointers.
9. Scalar widening is allowed only for declared safe cases: integer to number,
   and enum member to display string only when the value space is declared in
   the known rule schema and that known rule marks it display text. Unknown
   schemas cannot define an admissible enum member set.
10. Object-to-string, array-to-string, arbitrary-JSON-to-string, and
    number-to-date-string widening are forbidden.

Nearest-type scoring follows artifact 07:

| Score component | Weight |
|---|---:|
| `composition_kind` match | 100 |
| compatible required pointer overlap | 10 per pointer |
| compatible optional pointer overlap | 2 per pointer |
| render annotation similarity | 0-20 |
| namespace-prefix similarity | 0-5 |

Tie-break order is total score, kind match, required overlap, optional overlap,
annotation similarity, then lexicographically smaller type id.

### §6. Raw-payload exclusion contract

Dropped payload values must never appear in:

- visible text;
- `data-*` attributes;
- hidden inputs;
- tooltips;
- Gutenberg block attributes;
- HTML comments;
- hydration state;
- REST preload state;
- serialized JS props;
- inspector/debug panels;
- console logs;
- audit detail;
- diagnostics;
- snapshots written to post meta.

The implementation should include a negative fixture with sentinel values in
dropped fields and assert that the serialized projected result contains none of
those sentinel strings.

### §7. Banner contract

Every fallback render carries a non-dismissible banner:

```text
Rendered as nearest known type — payload may be incomplete.
```

This is the DOS-570 product-facing copy. It intentionally does not expose:

- original unknown type id;
- selected known type id;
- dropped pointer names;
- sensitivity labels;
- pipeline words such as enrichment, schema projection, substrate, or LLM.

Selected known type remains in diagnostics and audit for support/QA. Any
user-visible copy change requires product/design review before implementation.

### §8. Trust/provenance behavior for fallback

Fallback is a presentation degradation, not a claim rewrite.

- `claim_refs` are preserved exactly.
- `ProvenanceRef` is preserved exactly.
- W4-D must not dereference either to reconstruct dropped payload fields.
- The visible fallback block's trust band is capped at `needs_verification`.
- Source-role fields that survive projection render with ADR-0105 source
  attribution and ADR-0108 actor-filtered provenance treatment.
- For `Actor::SurfaceClient`, `ProjectedBlock` content routes through W4-B §16
  requester-scope projection before render. Out-of-scope fields are dropped by
  the same path as sensitivity-blocked fields, with no placeholder text.
- `ComputedFrom` and `DisplayOnly` fields with claim refs render only when the
  requester can read every referenced claim. Partial visibility drops the field.
- If provenance resolution fails, the block degrades visibly; it never renders
  as fully trusted.
- The preserved claim refs may still resolve to their real claim-level trust in
  an "about this" affordance. The block itself remains visibly degraded.

### §9. Diagnostics and audit intents

Diagnostics default to counts, not names.

`ProjectionDiagnostic` must include this field list verbatim:

```rust
pub struct ProjectionDiagnostic {
    pub diagnostic_kind: DiagnosticKind,
    pub composition_id: CompositionId,
    pub composition_version: u64,
    pub original_type_id: Option<String>,
    pub selected_known_type_id: Option<String>,
    pub block_id: Option<BlockId>,
    pub reason: DiagnosticReason,
    pub dropped_pointer_count: u32,
}
```

`DiagnosticReason` is a closed enum, not a free-form string. It must cover the
reasons emitted by this packet: `unknown_block_type`,
`unknown_block_cap_exceeded`, `unknown_admitted_field`, `sensitivity_blocked`,
`out_of_scope`, `unknown_role`, `missing_claim_ref`, `ambiguous_receiver`, and
`fallback_degraded_without_receiver`.

Diagnostics never include dropped pointer names or dropped values. Audit detail
may record whether a policy would allow pointer names, but the V3 detail schemas
below carry counts and a boolean only; adding actual names requires a later
schema version and product/security review. Dropped values never appear.

The projector returns `Vec<AuditIntent>` and does not call
`emit_surface_audit(...)`. This keeps `abilities-runtime` independent from the
app crate, where `src-tauri/src/audit_log.rs` lives. The caller, typically a
service-layer composition/surface path or `src-tauri/src/bridges/surface_client.rs`,
drains intents in stable order and calls `emit_surface_audit` once per intent.
The actor variant passed to that helper must match the calling surface.
SurfaceClient requires `wp_user_id`; audit emission fails if the actor lacks it.
That failure is returned by the caller's operation, not by `ProjectionError`.

Detail payloads exclude claim text, pointer values, source excerpts, customer
names, emails, and all PII. The caller maps `AuditIntent.category` into
`AuditFields::new(category, detail)` and attaches `wp_user_id` only for
`Actor::SurfaceClient`. Command handlers do not write audit rows directly.

`custom_block_fallback_applied` returns one `AuditIntent` per fallback block
with this exact JSON detail schema. The old ambiguous `cap_applied` boolean is
not emitted:

```jsonc
{
  "schema_version": 1,
  "composition_id": "string",
  "composition_version": "u64|null",
  "block_id": "string",
  "block_index": "u32",
  "original_type_id": "string",
  "selected_known_type_id": "string",
  "projected_pointer_count": "u32",
  "dropped_pointer_count": "u32",
  "pointer_names_included": "boolean",
  "composition_cap_state": "within_cap|cap_exceeded",
  "block_cap_action": "projected",
  "fallback_policy_version": "u32"
}
```

`custom_block_fallback_cap_exceeded` returns one `AuditIntent` per composition
when the composition exceeded the cap. A block affected by that excess is
represented by `block_cap_action`, not by overloading composition state:

```jsonc
{
  "schema_version": 1,
  "composition_id": "string",
  "composition_version": "u64|null",
  "unknown_block_count": "u32",
  "unknown_block_cap": "u32",
  "dropped_unknown_block_count": "u32",
  "dropped_block_ids": ["string"],
  "fallback_policy_version": "u32",
  "reason": "unknown_block_cap_exceeded"
}
```

### §10. Unknown-block cap

Default cap: **5 unknown blocks per composition render**.

The cap is enforced inside `project_composition_for_surface`. It is configurable
through substrate configuration, not per surface. A composition dominated by
unknown blocks is treated as a producer bug.

Behavior:

1. Walk blocks in stable composition order.
2. Apply fallback to the first `N` unknown custom blocks.
3. Drop excess unknown blocks from the rendered projection.
4. Preserve no raw payload from dropped excess blocks.
5. Return a `custom_block_fallback_cap_exceeded` `AuditIntent`.
6. Surface diagnostic count to W4-A.

W4-A may show a composition-level degraded-state notice using product-approved
copy, but it may not reveal dropped block payload.

### §11. Edit-routing metadata

Every projected block carries edit routes derived from `field_bindings`: field
path, role, claim refs, `feedback_allowed`, and a refusal reason when false.
`feedback_allowed` is true only for `FeedbackTarget` rows with enough receiver
information for W5-A's action. Refusal reasons use the closed
`EditRouteRefusalReason` enum from §1.1: computed, display-only,
source-without-target, missing claim ref, ambiguous receiver, sensitivity-blocked,
unknown_role, out-of-scope, and fallback-degraded-without-receiver. Unknown or
future roles are never converted into routable feedback.

W4-D publishes these routes; W5-A enforces nonce, concurrency, save-time diff,
and actual feedback application.

## Acceptance criteria lifted into DOS-570 (V3)

### Implementation

1. **No migration.** W4-D ships pure substrate logic and tests; no schema
   migration or slot reservation.
2. **Every known `BlockType` has a `BlockProjectionRule`.** CI/unit test fails
   for missing non-`Custom` variants.
3. **Rules publish admitted JSON-pointer fields** with sensitivity tier and
   allowed-surface metadata.
4. **Rules publish edit-routing metadata** derived from W4-B `BindingRole`.
5. **Unknown `BlockType::Custom` resolves by declared schema overlap and
   nearest-known-type intersection.**
6. **Selection uses declared schemas only, never runtime payload values.**
7. **Projection copies only admitted intersected pointers.** No object/array
   sibling copy-through.
8. **Missing unknown schema renders generic `dailyos/text` with empty payload.**
9. **No schema intersection renders generic `dailyos/text` with empty payload.**
10. **Unknown-block payload fields are never rendered raw** in DOM, JS,
    Gutenberg attributes, REST preload, tooltips, logs, diagnostics, or audit.
11. **`claim_refs` are preserved exactly** for applied fallback blocks.
12. **`ProvenanceRef` is preserved exactly** for applied fallback blocks.
13. **Fallback MUST NOT dereference `claim_refs` or `ProvenanceRef`** to
    reconstruct dropped payload fields.
14. **Fallback banner renders with exact DOS-570 copy:** "Rendered as nearest
    known type — payload may be incomplete."
15. **Banner is non-dismissible** for v1.4.2.
16. **Fallback trust band is capped at `needs_verification`.**
17. **Source-role fields render with provenance and trust band** per
    ADR-0105/0108 after sensitivity gating.
18. **ComputedFrom fields render only after every-claim scope checks** and are
    not feedback-routable.
19. **DisplayOnly fields render read-only, expose no edit affordance, and with
    non-empty claim refs pass the same every-claim scope gate as ComputedFrom.**
20. **FeedbackTarget fields publish explicit feedback receivers** for W5-A only
    when receiver metadata is present.
21. **W5-A refuses feedback routing** for ComputedFrom, DisplayOnly, Source-only,
    unknown_role, missing receiver, zero-ref receiver, or ambiguous receiver
    routes.
22. **Unknown-block count cap defaults to 5** and is enforced inside
    `project_composition_for_surface`, not by W4-A orchestration.
23. **Excess unknown blocks are dropped by the composition API** from rendered
    projection after cap.
24. **Projector returns `custom_block_fallback_applied` AuditIntent** once per
    fallback block using the exact §9 detail schema; the caller emits it through
    `emit_surface_audit(...)`.
25. **Projector returns `custom_block_fallback_cap_exceeded` AuditIntent** once
    per composition using the exact §9 detail schema when the composition
    exceeds the cap; the caller emits it through `emit_surface_audit(...)`.
26. **Diagnostics include counts by default.**
27. **Dropped pointer names do not appear in V3 diagnostics**; future schemas
    that add names require non-sensitive policy plus product/security review.
28. **Dropped payload values never appear in diagnostics or audit.**
29. **Projection result includes `fallback_policy_version: u32`** so render caches
    can invalidate when fallback rules change.
30. **W4-A renderer consumes `ProjectedComposition`** and has no independent
    nearest-known, admitted-field, cap, or edit-route logic.
31. **Future MCP/CLI/Tauri surfaces can call the same composition API.**
32. **All audit writes flow through services/existing audit helpers**, not
    command-handler DB writes; `abilities-runtime` only returns `AuditIntent`
    values and contains no dependency on `src-tauri/src/audit_log.rs`.
33. **No product-copy drift:** banner copy changes require product/design review.

### Negative fixtures

34. **`dos570_fixture_1_unknown_sensitive_payload_dropped.rs`** — unknown block
    contains sentinel sensitive fields; rendered result, diagnostics, audit, and
    serialized props contain none of the dropped values.
35. **`dos570_fixture_2_refs_preserved.rs`** — fallback preserves `claim_refs`
    and `ProvenanceRef` exactly while dropping non-admitted payload.
36. **`dos570_fixture_3_no_schema_generic.rs`** — no unknown schema produces
    generic `dailyos/text`, empty payload, banner, preserved refs.
37. **`dos570_fixture_4_no_intersection_generic.rs`** — declared schema exists
    but has zero eligible intersection; no invented text.
38. **`dos570_fixture_5_array_projection.rs`** — `/items/*/title` projects but
    `/items/*/private_note` does not.
39. **`dos570_fixture_6_unsafe_widening_rejected.rs`** — object/array/arbitrary
    JSON values never stringify into rendered text.
40. **`dos570_fixture_7_cap_exceeded.rs`** — calling
    `project_composition_for_surface` on 9 unknown blocks with cap 5 returns 5
    fallback blocks, drops 4, and returns the cap `AuditIntent`; a caller-path
    companion drains that intent through `emit_surface_audit` and verifies the
    written audit record.
41. **`dos570_fixture_8_diagnostics_redaction.rs`** — sensitive dropped pointer
    names are omitted; counts remain; Actor SurfaceClient scopes drop
    out-of-scope fields exactly like sensitivity-blocked fields.
42. **`dos570_fixture_9_binding_role_matrix.rs`** — Source, ComputedFrom,
    DisplayOnly, FeedbackTarget, and unknown/future-role fail-closed handling
    dispatch exactly per §4.
43. **`dos570_fixture_10_source_without_feedback_target_refused.rs`** — source
    renders provenance/trust, but edit route refuses feedback.
44. **`dos570_fixture_11_computed_from_refused.rs`** — N:M derived field renders
    only when all source claims are in scope; W5-A fake router refuses correction.
45. **`dos570_fixture_12_display_only_no_affordance.rs`** — display/layout field
    with claim refs uses the same scope gate, renders read-only, and emits no
    feedback route.
46. **`dos570_fixture_13_feedback_target_routes.rs`** — explicit receiver
    produces a routable edit route with claim id, claim version, and field path;
    FeedbackTarget with 0 claim_refs is refused and exposes no nonce affordance.
47. **`dos570_fixture_14_policy_version_cache_key.rs`** — changing rule version
    changes projected cache key/signature input without changing claim state.

### CI invariants

48. **Known BlockType coverage gate:** every non-`Custom` enum variant has one
    `BlockProjectionRule`.
49. **No catch-all admitted field gate:** tests fail if a rule admits arbitrary
    object payload or raw JSON stringify fallback.
50. **Dropped-value absence gate:** sentinel fixture asserts projected DTO JSON
    and `AuditIntent.detail` / emitted audit detail exclude dropped values.
51. **Role-routing gate:** `ComputedFrom`, `DisplayOnly`, unknown_role, and
    FeedbackTarget with 0 claim_refs cannot produce `feedback_allowed = true`.
52. **L1 commands green:** `cargo clippy -- -D warnings && cargo test && pnpm
    tsc --noEmit`.
53. **W4-B sequencing gate:** section 53 requires the W4-D implementation branch
    to declare a compile-time assertion that `BindingRole::FeedbackTarget` and
    `Block.field_bindings` resolve before any W4-D projector file builds.
54. **Asymmetric schema-evolution gate:** section 54 requires producer payload
    fields not in the registered admitted set for the running substrate version
    to be dropped, audited as `unknown_admitted_field`, and never raw-rendered.
    Forward-compat is asymmetric: unknown input fields are safe to ignore, but
    new admitted fields require classification and a `fallback_policy_version`
    bump.
55. **Surface scope projection gate:** ProjectedBlock content for
    `Actor::SurfaceClient` routes through W4-B §16 scopes; out-of-scope fields
    are dropped exactly as sensitivity-blocked fields.
56. **Public re-export gate:** consumers can import `EditRoute`,
    `ProjectionError`, `ProjectionDiagnostic`, and `AuditIntent` from the
    existing abilities module path without reaching into private modules.
57. **Invalid producer output gate:** `ProjectionError::InvalidProducerOutput`
    carries only the closed `ProducerOutputInvalidReason` enum from §1.1; no
    arbitrary string reasons or catch-all variants.
58. **Audit boundary gate:** projection tests assert the projector returns
    `AuditIntent` values, and caller-path tests assert services or
    `surface_client.rs` are responsible for `emit_surface_audit(...)` failures.

## Intelligence Loop fit

### 1. Claim model

W4-D does not introduce new claim tables or claim types. It consumes existing
claim refs and W4-B field bindings. Source-role and FeedbackTarget fields are
claim-bound with explicit `claim_id`, `claim_version`, and `field_path`; they are
not ad-hoc display-only data.

### 2. Provenance + trust

Source fields render through ADR-0105/0108 provenance and trust-band rules.
Fallback preserves `ProvenanceRef` and caps visible block trust at
`needs_verification`. Sensitivity metadata on admitted fields participates in
render/drop decisions.

### 3. Signals + invalidation

Fallback is read-side derived state and emits no claim-mutation signal. Render
caches must key on `composition_version`, `block_registry_version`, and
`fallback_policy_version`. Rule changes invalidate derived projections and W4-C
signature inputs, but do not mutate underlying claims.

### 4. Runtime + surfaces

`project_composition_for_surface` is callable from W4-A WordPress, future
MCP/CLI renderers, and Tauri surfaces. Surface behavior differs only in native
paint; the `ProjectedComposition`, raw-payload exclusion, trust cap, scope
projection, and edit routes are the same.

### 5. Feedback loop

User corrections, dismissals, corroborations, and contradictions flow only when
W5-A receives an explicit `FeedbackTarget` route and a valid W4-E nonce. Feedback
then mutates claim state through existing services. Fallback audit/cap intents
are observability, not claim feedback.

## Edge cases

- Empty payload, missing schema, or no schema intersection: banner plus empty
  generic payload; preserved refs; no invented text.
- Schema has only sensitive fields: fields are dropped and diagnostics stay count-only.
- Array field with mixed item shapes: rebuild only declared compatible item pointers.
- Duplicate role rows are additive; duplicate FeedbackTarget rows are ambiguous
  unless the normalized receiver set is identical.
- FeedbackTarget with zero claim refs is not claim-correctable in v1 and exposes no nonce-consuming affordance.
- Source binding without `ClaimRef.field_path` fails producer-output validation.
- Provenance ref cannot resolve: show provenance-unavailable treatment; do not
  synthesize provenance from payload.
- Registry changes mid-render use one atomic snapshot for the full composition.
- Custom type becoming known after upgrade uses the known rule on next render.
- Cap excess blocks are dropped with an audit intent; no hidden payload placeholders.
- Surface override of banner copy/admitted fields is a contract violation.

## Rollout risks

1. **Renderer-local drift.** W4-A may reimplement fallback in PHP. Mitigation:
   renderer consumes `ProjectedComposition` and tests assert no raw-payload
   branch.
2. **Banner-copy split.** ADR example includes type id; Linear pins generic copy.
   Mitigation: visible copy follows Linear; type id stays diagnostic-only.
3. **Source vs feedback confusion.** Source is claim-bound but not necessarily
   editable. Mitigation: only FeedbackTarget enables feedback route.
4. **Sensitive diagnostics.** Pointer names can leak. Mitigation: counts by
   default; names require explicit non-sensitive policy.
5. **Permissive existing skeleton.** Mitigation: harden registry, sensitivity,
   audit-intent, cap, and sentinel absence tests before merge.
6. **Cache staleness.** Mitigation: include `fallback_policy_version` in cache
   and signature inputs.

## Interlocks with W4 stage-2 + downstream

| Consumer | What it needs from W4-D | Status after W4-D merge |
|---|---|---|
| **W4-A renderer (DOS-572)** | Safe `ProjectedComposition`, exact banner, trust cap, no raw payload, edit routes for saved block attrs | Renderer can render unknown blocks without local policy |
| **W5-A feedback (DOS-573)** | Role-dispatched edit routes, FeedbackTarget receiver metadata, refusal reasons | Save handler can route without guessing from Gutenberg diffs |
| **W4-C tamper (DOS-569)** | Stable projected bytes and policy version for signature/cache inputs | Signature covers post-policy projection, not raw unknown payload |
| **W4-E nonce (DOS-571)** | Field path and feedback action only for FeedbackTarget routes | Nonce binding stays claim-field precise |
| **Future MCP/CLI/Tauri surfaces** | Same substrate projector and diagnostics | No surface-local privacy policy fork |

## What W4-D explicitly does NOT own

- **W4-B field topology.** `FieldBinding`, `BindingRole`, `ClaimRef.field_path`,
  and version watermarks are W4-B. W4-D inherits the surfaces and compile-time
  checks for their presence; it does not define them.
- **W4-A rendering code.** Gutenberg PHP/JS consumes W4-D output.
- **W5-A feedback application.** W4-D publishes routes; W5-A applies feedback.
- **W4-C signatures/quarantine.** W4-D supplies projected bytes and policy
  version; W4-C signs/verifies.
- **W4-E nonce lifecycle.** W4-D marks eligible fields; W4-E binds user
  presence to action/field/version.
- **New persistent registry store.** Out of scope unless a successor L0 packet
  approves a migration.
- **Product redesign of fallback UX.** Exact copy is pinned; visual treatment is
  W4-A/design unless copy changes.
- **Surface endpoints.** W4-B §17 `wp_user_id` body/session binding and W4-B §37
  `surface_client.rs` route placement apply to endpoint-owning work. W4-D owns no
  `/v1/surface/*` endpoint and has no direct bridge obligation.

## Open questions

All resolved in V3 unless reviewers reopen:

| ID | Resolution |
|---|---|
| Q1 (migration slot) | No migration; pure substrate logic |
| Q2 (banner copy) | Linear DOS-570 generic product copy wins over ADR example with type id |
| Q3 (unknown-block cap) | Default 5 per composition render; configurable substrate-side |
| Q4 (pointer names in diagnostics) | V3 diagnostics are count-only; names require a later schema and review |
| Q5 (Source editability) | Source renders provenance/trust; only FeedbackTarget routes feedback |
| Q6 (0-ref FeedbackTarget) | Not claim-correctable in v1; W5-A refuses claim feedback |

## Linear dependency edges

- DOS-570 is blocked by DOS-556 (W1-E Composition types) — clear once W1-E is
  merged in the implementation branch.
- DOS-570 is blocked by DOS-567 (W4-B V8 field topology).
- DOS-570 blocks DOS-572 (W4-A renderer).
- DOS-570 blocks DOS-573 (W5-A feedback router).
- DOS-570 is related to DOS-546 parent spike.
- W0-A ADR-0130 amendment is a documentation dependency; implementation follows
  the landed ADR/amendment plus this packet's Linear-copy resolution.

## L0 reviewer panel — required runners

- `/plan-eng-review` for architecture, rule registry, cache invalidation, and
  W4-B/W4-A/W5-A interlocks.
- `/cso` because this is a privacy/trust-boundary feature preventing raw payload
  disclosure.
- `/codex challenge` for adversarial review of projection leaks and role-routing
  ambiguity.
- `/plan-devex-review` because W4-D publishes a cross-surface API consumed by WP
  and future MCP/CLI/Tauri surfaces.
- `/plan-design-review` is conditional: required only if banner copy, visible
  fallback treatment, or user-facing terminology changes from Linear DOS-570.

Unanimous APPROVE from the required panel closes L0.

## V2 to V3 reviewer fold record

Cycle 2 status: eng, cso, and devex approved V2; codex returned conditional
findings N1/N2 plus minor folds. V3 resolves those conditions in-place:

- **Codex N1 HIGH:** The audit emission contract no longer requires
  `abilities-runtime` to call `emit_surface_audit`. The projector returns
  `Vec<AuditIntent>`; app/service callers emit through the existing helper.
- **Codex N2 MEDIUM:** `ProjectionDiagnostic` now carries the composition and
  original-type fields required by §9, with a closed `DiagnosticReason`.
- **Eng P3-new-1:** `ProjectionError::InvalidProducerOutput` now carries a
  closed `ProducerOutputInvalidReason` enum.
- **Eng P3-new-2:** `EditRouteRefusalReason` is pinned in the DTO sketch.
- **Devex P3-DX-1:** `EditRoute`, `ProjectionError`, `ProjectionDiagnostic`,
  and `AuditIntent` are listed in the public re-export contract.
- **Devex P3-DX-2:** The old audit error variant is removed; audit emission
  failures belong to the caller path that drains intents.

## Acceptance for L0 closure

This packet is L0-approved when:

1. eng + cso + devex APPROVE and codex conditional findings are folded into
   V3 or a successor cycle.
2. DOS-570 issue description links this packet as the lifted contract.
3. Reviewers agree W4-D has no migration footprint.
4. Reviewers agree banner visible copy follows Linear's generic sentence.
5. Reviewers agree BindingRole dispatch matrix in §4 is the W5-A routing source.
6. Reviewers agree unknown-block cap default is 5 and substrate-side.
7. Reviewer findings are folded into successive packet versions and dated.

W4-D implementation may start as soon as L0 closes and W4-B's V8 surfaces are
available on the implementation branch. W4-A renderer starts only after W4-D
merges, because W4-A acceptance consumes this substrate policy directly.
