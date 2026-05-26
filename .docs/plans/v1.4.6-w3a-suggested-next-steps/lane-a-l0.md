---
ticket: DOS-298
title: "v1.4.6 W3-A — Suggested Next Steps block contract"
parent_plan: ../v1.4.6-waves.md
fork_sha: e73ac8949b0df46c33b441fc14737cee2a9c3a73
branch: codex/v1.4.6-w3a-suggested-next-steps
status: L0 — plan hardening, cycle 0 draft
authors: James Giroux, Claude
related:
  - ADR-0129 (Composable Surfaces — WordPress Studio as Primary Surface)
  - ADR-0130 (Surface-Independent Composition Contract)
  - ADR-0105 (Provenance as First-Class Output)
  - ADR-0108 (Provenance Rendering and Privacy)
  - ADR-0111 (Surface-Independent Ability Invocation)
  - DOS-329 (RecommendationClaim contract — landed)
  - DOS-333 (cross-surface embedding — W3-B, blocks on W3-A)
  - DOS-332 (recommendation feedback semantics — W4-A consumer)
---

# v1.4.6 W3-A — Suggested Next Steps block contract (DOS-298)

## §0 Frame

W3-A ships **two coupled deliverables in one PR**:

1. A new **`SurfaceClient`-allowed read ability** under `src-tauri/abilities-runtime/src/abilities/recommendations/` that projects typed recommendations for a subject into a privacy-safe, render-ready list.
2. A new **Gutenberg block** `dailyos/suggested-next-steps` under `wp/dailyos/blocks/suggested-next-steps/` that calls (1) via the `dailyos_runtime_client_for_block` filter and renders per-claim trust band, why-this-now, evidence/provenance summary, and feedback affordances.

The block **never reads** `surfacing_decisions`, `triggers_log`, or any other W2 table directly. The ability is the only seam. This is the producer-boundary invariant the wave plan codifies (`v1.4.6-waves.md:541–558`).

**Reviewer set for this packet (per `v1.4.6-waves.md:435–449` + memory rules):**

- `/codex challenge` — default planning challenger
- `/plan-eng-review` — default planning reviewer
- `/plan-design-review` — design-mandatory for W3-A per plan
- `/plan-devex-review` — new ability contract + producer/projection boundary
- `ce-learnings-researcher` — K-in (parallel, mandatory)
- **WP-skill-grounded panel** (5th slot per memory rule for `wp/dailyos/blocks/**` touches) — block.json/render.php verification

Unanimous required. Path-α findings (theoretical hardening, scope adjacent) → Codebase Maintenance project; substrate plan does not block on them.

---

## §1 Visual parity matrix

> **WP-wave rule (memory):** every WP wave packet must carry a parity matrix in §1. Visual parity is BLOCKING at L0 — not deferred to L4 verification.

| Surface slot                                                  | Canonical reference                                  | Nearest existing primitive                              | W3-A action                                                                                                                       |
|---------------------------------------------------------------|------------------------------------------------------|---------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------|
| Account detail — "Suggested next steps for this account"      | **NONE — gap**                                       | `PostMeetingIntelligence_actionItem` + `Suggested` pill | Author reference HTML in `.docs/design/reference/surfaces/account.html` as part of W3-A L0 close-out, then translate to render.php |
| Project detail — "Suggested next steps for this project"      | **NONE — gap**                                       | Same as Account                                         | Same                                                                                                                              |
| Person detail — "Suggested topics for next 1:1"               | **NONE — gap**                                       | `StakeholderGallery` + "Suggested" treatment            | Same                                                                                                                              |
| Meeting prep — "Suggested topics"                             | Adjacent: `meeting.html:235` `actionItemSuggested`   | `PostMeetingIntelligence_actionItem` row + Suggested pill | Compose from existing primitives, no new tokens                                                                                 |
| Work surface — "Recommendations" filter chip                  | **NONE — gap**                                       | Work filter chips (existing)                             | W3-B owns the chip placement; W3-A's block renders inside the chip-filtered list                                                  |
| Daily Briefing — block embedded into briefing composition     | Adjacent: `briefing.html` open-loop chapter style    | `DailyBriefing.module.css` open-loop row patterns       | Block reuses chapter-section heading rule (`ChapterHeading_*` family) per `meeting-recommended-actions` precedent                 |

**Resolution rule:** W3-A's PR MUST include reference HTML additions for the four "gap" rows above. The block's `render.php` and accompanying CSS module then translate from the reference. New theme.json tokens added only where existing tokens cannot express the visual; default posture is **token reuse**, not new primitives (memory: "audit chrome overlap before promoting new DS patterns").

**Tokens expected to reuse (from `wp/dailyos/theme/theme.json` and existing reference module CSS):**

- `ChapterHeading_*` family (section heading + rule)
- `meeting-intel_chapterSection`, `meeting-intel_chapterSection--empty` shape (empty chip rules)
- `dailyos-empty-chip` (empty-state visible chip — `meeting-recommended-actions:171`)
- `editorial-reveal` (reveal-on-scroll wrapper)
- TrustBand primitive (per ADR-0108 actor-filtered rendering)

**Tokens to **propose adding** (only if reference work confirms a gap):**

- `suggested-next-steps_*` family (row, why-this-now caption, action button row) — names finalized after reference HTML lands

L0 design review (`/plan-design-review`) explicitly owns the reference-HTML-first decision.

---

## §2 Scope and producer authority

### In scope

| File / surface                                                                                                                  | Owner   |
|----------------------------------------------------------------------------------------------------------------------------------|---------|
| `src-tauri/abilities-runtime/src/abilities/recommendations/list_suggested_next_steps.rs` (or extension of `mod.rs`)              | W3-A    |
| `src-tauri/abilities-runtime/src/abilities/recommendations/contracts.rs` — additive: list-projection DTO                          | W3-A    |
| `src-tauri/src/services/recommendations/projection.rs` (new) — substrate-side projection from `surfacing_decisions` + claim store | W3-A    |
| `src-tauri/src/services/context.rs` — wire new read handle (additive trait method)                                                | W3-A    |
| `tools/dailyos-abilities.json` — regen after new ability lands                                                                   | W3-A    |
| `wp/dailyos/blocks/suggested-next-steps/block.json`                                                                              | W3-A    |
| `wp/dailyos/blocks/suggested-next-steps/render.php` + `render-functions.php`                                                     | W3-A    |
| `wp/dailyos/blocks/suggested-next-steps/style.css` (block CSS module)                                                            | W3-A    |
| `wp/dailyos/theme/theme.json` — additive only, with token-name resolution from L0 design review                                  | W3-A    |
| `.docs/design/reference/surfaces/{account,project,person}.html` — author reference sections (per §1)                              | W3-A    |
| `wp/dailyos/tests/blocks/suggested-next-steps/` (PHPUnit or render-fixture tests)                                                 | W3-A    |
| Integration fixture proving WP-block-calls-ability path                                                                          | W3-A    |

### Out of scope

- W2 surfacing policy table writes (W2-A, merged)
- W2 trigger scanner (W2-B, merged)
- `score_salience` ability (existing; **MUST NOT** broaden actor set — see §3.3)
- Cross-surface embedding (DOS-333 / W3-B; blocks on W3-A merge)
- Feedback **semantics** (DOS-332 / W4-A) — W3-A wires affordance UI + call sites; W4-A lands the feedback ability proper (see §9 cross-wave coordination)
- MCP exposure (deferred to v1.4.7+ per `v1.4.6-waves.md:391`)
- Engagement telemetry (W4-C)

### Producer authority restatement

Per `v1.4.6-waves.md:541–558`:

> Producer gate: before W3 or later user-facing work starts, the lane L0 must identify the exact ability producer / projection that returns recommendation lists, surfacing state, trust, provenance, caveats, and receipt refs. If that ability is not present, W3 blocks rather than reading `surfacing_decisions` directly.

**The ability is not present.** W3-A authors it. This packet is the producer-boundary identification.

---

## §3 Substrate contract — new SurfaceClient ability

### §3.1 Ability identity

- **Name:** `list_suggested_next_steps`
- **Category:** `Read`
- **Allowed actors:** `[User, System, SurfaceClient]` (no MCP — MCP exposure deferred to v1.4.7+)
- **Required scopes:** `["read.recommendations"]` (same scope namespace as existing `score_salience`; v1.4.7+ MCP wrap will use `dailyos.read.recommendations` per `v1.4.6-waves.md:391`)
- **`may_publish`:** false (read-only)
- **`client_side_executable`:** false (substrate-side execution per ADR-0111)
- **`mcp_exposure`:** `None`
- **`composes`:** internally reads `surfacing_decisions` (W2-A authoritative), `claim_store`, and `claim_receipt` projection; emits actor-filtered list
- **`signal_policy.emits_on_output_change`:** `[]` (read-only ability)
- **Schema version:** `1`

### §3.2 Input

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSuggestedNextStepsInput {
    pub schema_version: u32,                 // pinned to 1 for v1.4.6
    pub subject: Option<SubjectRef>,         // None = global feed (Work surface), Some(entity) = per-entity surface
    pub surface: ClaimReceiptSurfaceContext, // routes audience filter through Shared Receipt DTO
    pub max_items: Option<u8>,               // optional cap; default + ceiling per ADR-0130 §2 size guard
}
```

Notes:
- `SubjectRef` is the existing `abilities_runtime::abilities::provenance::subject::SubjectRef` enum (Account, Project, Person, Meeting, Global, User, etc.)
- `ClaimReceiptSurfaceContext` is reused from `src-tauri/abilities-runtime/src/abilities/claim_receipt/contracts.rs:32` so receipt rendering and surface-audience filtering share one type
- `max_items` ceiling: **8** (W3-A L0 default; tuned after L4 surface QA)

### §3.3 Output

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListSuggestedNextStepsResponse {
    pub schema_version: u32,
    pub items: Vec<SuggestedNextStepItem>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedNextStepItem {
    pub claim_id: ClaimId,                       // stable id for feedback callbacks
    pub headline: String,                        // product-safe (no PII bleed from raw evidence text)
    pub why_this_now: String,                    // pre-rendered text from WhyThisNow.text (W2-A produces)
    pub primary_factor: SalienceFactorKind,      // for chip / icon rendering only — NOT full factor breakdown
    pub recommended_action: RecommendedActionView, // privacy-safe view (see §3.4)
    pub trust_band: TrustBand,                   // for trust-band primitive rendering (ADR-0108)
    pub receipt: ClaimReceiptSnapshot,           // Shared Receipt DTO per v1.4.4 invariant (v1.4.6-waves.md:543)
    pub feedback_state: FeedbackState,           // for affordance state (pending vs decided)
    pub conversion_state: ConversionState,       // for "already converted" affordance state
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum RecommendedActionView {
    ScheduleMeeting { entity_label: String, when_window: String }, // no raw rationale; rationale flows through why_this_now
    SendMessage    { entity_label: String, channel: String },
    ReviewClaim    { claim_label: String },
    UpdateRecord   { entity_label: String, field_label: String },
    InvestigateChange { entity_label: String, change_label: String },
    Custom         { action_label: String },                      // payload dropped per ADR-0130 §3.1 fallback rule
}
```

**Privacy boundaries (load-bearing — codex security review hits this hardest):**

- `RecommendedActionView` is the **projection** of `RecommendedAction` for surfaces — **never** ships the raw enum payload to a `SurfaceClient`. Entity IDs become resolved labels via the same redaction layer the receipt projection uses (`ClaimReceiptRedactionLevel`).
- `SuggestedNextStepItem` carries `primary_factor` only, **never** the full `Vec<SalienceFactor>`. Full factor breakdown stays inside `score_salience` (User/System actors only). The hidden-ability contract is preserved (`v1.4.6-waves.md:941`).
- `provenance` flows through `ClaimReceiptSnapshot.provenance` (the receipt's resolved view), not as a raw envelope on the item.
- `feedback_state` and `conversion_state` ship as-is — they are part of the claim's lifecycle and are surface-safe.

**Hidden-ability sweep test (codex security panel):** the W3-A integration fixture MUST include a test that, given a SurfaceClient invocation, the response JSON does NOT contain any of: `factors` array, raw `RecommendedAction` payload fields not surfaced in `RecommendedActionView`, raw evidence source paths, prompt/provider-looking strings.

### §3.4 Substrate-side projection rules

The new `services::recommendations::projection::list_suggested_next_steps()` function:

1. **Filters** `surfacing_decisions` to `tier ∈ {Critical, Notable, Background}` and `kind = Render` (Quiet, Defer, Suppress excluded for surfaces — `v1.4.6-waves.md:544`).
2. **Joins** with the claim store for `RecommendationClaim` rows matching the `subject` filter.
3. **Orders** by salience `total` descending, then by `created_at` descending as tiebreaker.
4. **Truncates** to `max_items` (default 8, ceiling 8).
5. **Maps** `RecommendedAction` → `RecommendedActionView` via redaction layer.
6. **Invokes** existing `claim_receipt` projection per claim to populate `receipt` field — re-using v1.4.4 Shared Receipt DTO, not authoring a parallel one (`v1.4.6-waves.md:543`).
7. **Returns** the privacy-safe response.

Salience scoring is NOT recomputed at projection time — it reads the already-stored evaluation from W1-B's path (`services/recommendations/salience.rs::score_salience`). The hidden-ability contract calls the same substrate read; this ability projects the surface-safe subset.

### §3.5 Provenance envelope

Output ships with full `Provenance` envelope per ADR-0105. Schema version `1` for the response payload. Provenance attribution:

- Subject: per-call (matches `input.subject` when present, else `SubjectRef::Global`)
- Field attributions for each item resolve to the originating `RecommendationClaim` provenance (via `ClaimReceiptProvenance` already produced in `ClaimReceiptSnapshot`)
- No new envelope authoring; reuse existing claim_receipt provenance plumbing

---

## §4 Block contract — `dailyos/suggested-next-steps`

### §4.1 `block.json`

```json
{
    "$schema": "https://schemas.wp.org/trunk/block.json",
    "apiVersion": 3,
    "name": "dailyos/suggested-next-steps",
    "title": "Suggested Next Steps",
    "category": "dailyos",
    "description": "Recommendations for the current subject. Renders typed RecommendationClaim list with per-claim trust band, why-this-now, evidence/provenance summary, and feedback affordances.",
    "supports": { "html": false, "reusable": false, "inserter": false },
    "usesContext": [ "dailyos/entityType", "dailyos/entityId" ],
    "attributes": {
        "maxItems":  { "type": "number", "default": 8 },
        "headingLabel": { "type": "string" }
    },
    "render": "file:./render.php"
}
```

Notes:
- `inserter: false` — block is inserted by composition (W3-B), not directly by editor
- `usesContext` mirrors `meeting-recommended-actions:9` precedent — block resolves entity from outer context provided by `dailyos/{account,project,person,meeting}-detail` and from a future `dailyos/work-surface` wrapper (W3-B scope)
- `headingLabel` attribute lets the parent surface override the section heading ("Suggested next steps for this account" vs "Suggested topics for next 1:1")
- `parent` constraint deliberately omitted at W3-A — W3-B configures it once embedding pages are confirmed

### §4.2 `render.php` shape (terse stub)

```php
<?php
declare(strict_types=1);
if ( ! defined( 'ABSPATH' ) ) { return ''; }
require_once __DIR__ . '/render-functions.php';
return dailyos_suggested_next_steps_render( $attributes ?? [], $block ?? null );
```

### §4.3 `render-functions.php` algorithm

Mirrors `meeting-recommended-actions/render-functions.php` precedent. Differences:

1. **Subject resolution.** Resolve `SubjectRef` from `$block->context`:
   - `dailyos/entityType = 'account'` + `dailyos/entityId = '...'` → `SubjectRef::Account(...)`
   - `'project'`, `'person'`, `'meeting'` → analogous
   - Missing context with `parent === 'dailyos/work-surface'` → `SubjectRef::Global` (or `SubjectRef::User` per W3-B confirmation)
   - All other missing context → empty chip `missing_subject_context`

2. **Ability call.** `$runtime_client->invoke_ability('list_suggested_next_steps', [...input...], $scope_set)` with `surface = surface_for_context(...)`.

3. **Error handling** per `meeting-recommended-actions:46–65`:
   - Runtime unavailable → empty chip `runtime_unavailable`
   - WP_Error / `ok === false` → empty chip `envelope_error`
   - Empty items list → empty chip `no_recommendations` (label varies by surface — "No suggested next steps." for accounts/projects, "No suggested topics." for person/meeting)

4. **Per-item rendering:**
   - Section heading: `editorial-reveal` + `ChapterHeading_*` family with `attributes.headingLabel` (or surface default)
   - Per-row: trust-band primitive (server-rendered per ADR-0108 audience filter from `receipt.trust`), headline, why-this-now caption, recommended-action view, action affordance ("Convert to action" / "Dismiss" / "Not useful")
   - Provenance summary inline-or-drawer: link to `evidence-drawer` block where it exists, otherwise inline truncated source list
   - Each affordance carries `data-claim-id="..."` + `data-feedback-kind="dismiss|accept|notUseful|tooNoisy|convert"` for the W4-A feedback handler (see §9)

5. **Always-visible empty chip** when `items === []`, per `v1.4.6-waves.md:544` invariant — "§10 invariant — never silent-hidden" (`meeting-recommended-actions:163`).

### §4.4 Style module

- New file: `wp/dailyos/blocks/suggested-next-steps/style.css`
- Class namespace: `suggested-next-steps_*` (matches `meeting-intel_*` precedent shape)
- Composes existing primitives where possible; new classes only where the parity matrix gap forces them
- Block stylesheet enqueue follows existing wp-block-themes pattern (registered via `register_block_type_from_metadata`)

### §4.5 Editor / view scripts

Per wave plan: `editor/view files only if the block needs interactive controls` (`v1.4.6-waves.md:942`). W3-A view script needs:

- **Client-side affordance handler** — POSTs to a feedback ability (W4-A) via the WP REST endpoint that bridges to the abilities runtime
- During the W4-A gap (see §9), the click handler is registered but **either** disabled (button shows `aria-disabled="true"` until feedback ability lands) **or** stubbed to a no-op endpoint that returns `200 not_yet_implemented`

W3-A authors the affordance markup + handler. The decision between disabled vs no-op stub is an L0 question (see §12).

---

## §5 Acceptance criteria

Plain-language ACs, traceable to the wave plan invariants and DOS-298 description.

1. **Ability ships and is registered.** `list_suggested_next_steps` appears in `tools/dailyos-abilities.json` (regen committed in same PR), and `AbilityRegistry::global_checked()` exposes it to `Actor::SurfaceClient` with scope `read.recommendations`. `cargo test recommendations::list_suggested_next_steps` passes.
2. **Block renders for the four entity surfaces.** Test fixture proves `dailyos/suggested-next-steps` inside `dailyos/{account,project,person,meeting}-detail` parents produces non-empty HTML with the expected class names when seeded recommendations exist for that subject.
3. **Empty state ships visible chip.** Each surface's empty render produces `dailyos-empty-chip` with a machine-readable reason (`no_recommendations`, `missing_subject_context`, `runtime_unavailable`, `envelope_error`). No silent hiding — `v1.4.6-waves.md:544`.
4. **Hidden-ability sweep passes.** Integration test asserts SurfaceClient JSON response contains no `factors` array, no raw `RecommendedAction` payload, no envelope copy. `score_salience` actor allowlist unchanged.
5. **Trust band renders per claim.** Each item's `receipt.trust.band` flows through the existing trust-band primitive; trust display matches the receipt's actor-filtered view (ADR-0108).
6. **Why-this-now is product-safe.** No raw evidence-source paths, no prompt/provider-looking strings, no PII bleed. Test asserts against a fixture with seeded sensitive content.
7. **Receipt is the Shared Receipt DTO.** Each item carries `ClaimReceiptSnapshot`, not a parallel receipt struct. `cargo clippy -- -D warnings` enforces (grep CI on `RecommendationReceipt|RecRow` per `v1.4.6-waves.md:543`).
8. **No W2 table reads from the block.** Static grep in CI: `wp/dailyos/blocks/suggested-next-steps/**` contains no references to `surfacing_decisions`, `triggers_log`, or any other W2 table name.
9. **No new migrations.** Empty migrations diff in the PR (per `v1.4.6-waves.md:464`).
10. **Reference HTML lands** for account/project/person surfaces — sections added to `.docs/design/reference/surfaces/{account,project,person}.html` matching the block's rendered shape. Visual parity matrix in §1 of this packet is updated from "gap" to "matches reference: lines X-Y".
11. **L4 surface QA runs BEFORE L2** per `v1.4.6-waves.md:440`. `/qa-only` evidence (screenshots of empty + non-empty + over-budget states) is attached to the Linear ticket before L2 dispatch.
12. **Cycle hygiene.** Full validation: `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit && phpunit (wp/dailyos/tests)`.

---

## §6 Test plan

### Unit (Rust)

- `list_suggested_next_steps` golden wire-shape fixture (mirrors `recommendation_contract_golden_wire_shape` test pattern in `services/recommendations/contracts.rs:441`)
- Hidden-actor denial: `Actor::McpClient` denied before reader
- Surface actor allowed with `read.recommendations` scope; denied without scope
- Subject filtering: `SubjectRef::Account(x)` returns only account-x recommendations; `None` returns global feed
- `max_items` ceiling respected (request 100, response capped at 8)
- Privacy-safe response: parametric test asserts no leakage of raw factors / raw payload / raw evidence sources / prompt strings

### Unit (PHP)

- `dailyos_suggested_next_steps_extract_items()` parser handles ability response shape (`response.ability.data.items`, `response.data.items`, fallback to raw)
- Empty chip variants render with correct reason codes
- Section heading uses `attributes.headingLabel` when set, surface default otherwise
- Affordance markup carries `data-claim-id` + `data-feedback-kind` for every item

### Integration

- End-to-end fixture: WP block render → runtime client filter → ability invocation → projection from seeded substrate → rendered HTML matches snapshot
- Snapshot tests for: empty state, single-item, max-items-truncated, over-budget-with-defer

### L4 surface QA (before L2)

- `/qa-only` walks Account, Project, Person, Meeting surfaces in the local WP install
- Captures screenshots for empty + 1 + 3 + 8 recommendation counts
- Verifies trust band rendering, why-this-now caption, feedback affordance presence (and disabled-state if W4-A gap holds)
- Attaches evidence to DOS-298 ticket comment before L2 dispatch

---

## §7 CI gate inputs (L1 deliverables — memory rule)

> Memory: "CI gate inputs are L1 deliverables, not verification steps." Every artifact below ships in the same PR as the feature.

| Artifact                                                                                          | Owner | Notes                                                            |
|---------------------------------------------------------------------------------------------------|-------|------------------------------------------------------------------|
| `tools/dailyos-abilities.json` regenerated to include `list_suggested_next_steps`                 | W3-A  | Run regen script in PR; commit alongside ability                 |
| Ability allowlist entry (SurfaceClient scope policy registry)                                     | W3-A  | If gated by a scope registry file, update in same PR             |
| Integration fixture (Rust → WP) end-to-end test                                                   | W3-A  | Lives in `wp/dailyos/tests/` per existing PHP test convention    |
| Block `style.css` enqueued via `register_block_type_from_metadata`                                | W3-A  | Required for visual parity gate                                  |
| theme.json token additions (if any) registered                                                    | W3-A  | Subject to L0 design review confirming gap                        |
| Reference HTML additions in `.docs/design/reference/surfaces/{account,project,person}.html`       | W3-A  | Visual parity gate                                                |
| W2-table-read grep guard (CI lint)                                                                | W3-A  | Add to existing lint script; fail PR if matches                  |
| Hidden-ability sweep test (`factors` / raw payload leakage)                                       | W3-A  | Privacy gate                                                      |
| L2-status declaration in commit messages                                                          | W3-A  | `L2-status: passed` (mandatory per memory + `.githooks/commit-msg`) |
| L4 evidence attached to DOS-298 before L2 dispatch                                                | W3-A  | `/qa-only` screenshots                                            |

---

## §8 Migration disposition

**No new migrations.** Per `v1.4.6-waves.md:464`: W3 row reads "(none expected — UI consuming W1+W2 contracts)". Next available slot v274 stays reserved for W4 lanes.

If L0 review surfaces an unexpected storage need, the wave plan amendment process applies (see Engineering Ladder K-channel) before claiming v274.

---

## §9 Cross-wave coordination

### W3-A → W3-B handoff

- W3-A merges first (`v1.4.6-waves.md:527`)
- W3-B (DOS-333) consumes the block + the ability; embeds across surfaces; adds work-surface filter chip wrapper
- W3-B does NOT modify ability contract or block contract — additive composition only

### W3-A → W4-A coordination (FEEDBACK GAP)

**This is the load-bearing coordination concern.** W4-A owns `services/recommendations/feedback.rs` and the feedback ability proper (`v1.4.6-waves.md:241`). W3-A renders feedback affordances. Two viable patterns:

| Option | Pattern | Tradeoff                                                                                                      |
|--------|---------|---------------------------------------------------------------------------------------------------------------|
| A      | Affordance markup + view.js handler render, but click is `aria-disabled="true"` + tooltip "Feedback coming in W4-A". | Cleanest privacy posture; honest UI. W4-A enables the handler in a follow-up PR. |
| B      | W3-A authors a **minimal feedback ability skeleton** (Pending → Decided{Dismiss} only, no echo signals); W4-A fills in semantics. | Affordance works at W3-A merge; W4-A scope grows but no UI churn.                 |
| C      | Coordinate so W4-A's `submit_recommendation_feedback` ability lands FIRST as a pre-W3 dependency.                    | Reorders waves — friction. Wave plan ordering says W3 < W4.                       |

**Recommendation:** **Option A** (disabled-with-tooltip).
- Honest about state of system
- Preserves W3-A as a clean producer-boundary PR (no scope leak)
- W4-A's job becomes "enable the existing UI" — minimal-surface UI work in W4

L0 panel decides. This is the single highest-leverage open question.

### W3-A → W4-C (engagement telemetry)

W4-C (DOS-462 / engagement.rs) wants to record `Rendered`, `Clicked`, `Dismissed`, `Ignored` signals against `RenderSurface`. The block's render path is a natural emission point. W3-A's view.js handler should fire a `Rendered` signal on mount and `Clicked` on affordance click — but this depends on the same W4 ability landing.

**Treatment:** Same as feedback — W3-A wires the call sites and disables them until W4-C lands. Track as L0 open question parallel to the feedback gap.

---

## §10 Out of scope (restatement)

- W2 surfacing policy / trigger scanner work (W2 merged)
- `score_salience` actor broadening (stays User/System only — preserved per `v1.4.6-waves.md:941`)
- Cross-surface embedding (W3-B)
- Feedback semantics + ability behavior (W4-A)
- Engagement telemetry behavior (W4-C)
- MCP exposure (v1.4.7+)
- Deviation baselines (W4-B)
- Recommendation eval harness (W5)
- Migration work (none in W3)

---

## §11 K-in citations

K-in grep results from `docs/solutions/` + `.docs/decisions/` (parallel `ce-learnings-researcher` pending; preliminary scan below):

| Source                                                                                                              | Relevance                                                                                                                                          |
|---------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------|
| ADR-0102 — Abilities as Runtime Contract                                                                            | All abilities follow `AbilityOutput<T>` shape with single envelope on the wrapper                                                                  |
| ADR-0105 — Provenance as First-Class Output                                                                         | Envelope lives once; field attributions reference into it                                                                                          |
| ADR-0108 — Provenance Rendering and Privacy                                                                         | Actor-filtered render; trust-band rendering rules; 64KB serialized cap                                                                             |
| ADR-0111 — Surface-Independent Ability Invocation                                                                   | `SurfaceClient` actor class; scope-based actor filtering; bridge-per-surface invocation                                                            |
| ADR-0125 — Claim Anatomy, Temporal Scope, Sensitivity, TypeRegistry                                                 | `RecommendationClaim` metadata: temporal=State, sensitivity=Internal, freshness=Medium, commit=Replace, allowed_actor=Agent (see contracts.rs:421) |
| ADR-0129 — Composable Surfaces                                                                                      | WP as primary surface; blocks are typed projections; WP MCP via Abilities API + MCP Adapter                                                        |
| ADR-0130 — Surface-Independent Composition Contract                                                                 | §3.1 custom-block fallback projection (relevant if W3-B uses Composition wrapper); §2 size guard                                                   |
| DOS-689 (Evidence Drawer block) — `wp/dailyos/blocks/evidence-drawer/`                                              | Server-rendered provenance summary + client toggle drawer pattern. W3-A links to evidence drawer where present.                                    |
| `wp/dailyos/blocks/meeting-recommended-actions/`                                                                    | The dominant block precedent — `usesContext` + runtime_client filter + scope filter + empty-chip-never-silent invariant                            |
| `services/recommendations/contracts.rs`                                                                             | RecommendationClaim type + RecommendedAction enum + SurfacingDecision + WhyThisNow                                                                 |
| `abilities-runtime/src/abilities/recommendations/mod.rs`                                                            | `score_salience` ability — hidden-from-clients invariant W3-A MUST preserve                                                                        |
| `abilities-runtime/src/abilities/claim_receipt/`                                                                    | Shared Receipt DTO that W3-A reuses for `SuggestedNextStepItem.receipt`                                                                            |

**No `docs/solutions/` entries found for "suggested next steps", "recommendation surface", "block producer", or "projection ability".** Gap is substrate-bound; surfacing/WP binding is W3-A scope.

`ce-learnings-researcher` will run in parallel during L0 review to confirm no hits I missed.

---

## §12 Open questions (L0 panel decides)

1. **Feedback affordance pattern** (load-bearing — see §9). Option A (disabled-with-tooltip), Option B (skeleton ability), or Option C (reorder waves)?
2. **theme.json token strategy.** Reuse existing tokens only (matrix §1), or add a `suggested-next-steps_*` token family? Design review decides; default is reuse.
3. **Work-surface subject resolution.** Work-surface block context resolves to `SubjectRef::Global` or `SubjectRef::User`? Affects projection filter behavior. Plan to confirm with W3-B L0.
4. **Reference HTML scope.** Does W3-A author full reference sections in account.html / project.html / person.html, or minimal reference snippets that W3-B fills in? Default: minimal sections that the block can be visually verified against; W3-B integrates them into composition.
5. **`headingLabel` default per surface.** Hardcode in render-functions.php or thread via `usesContext` from outer block? Default: hardcoded in render-functions.php with `dailyos/entityType` switch (matches `meeting-recommended-actions` precedent).
6. **`max_items` ceiling = 8.** Sane? If L4 surface QA on 8-item dense states regresses scannability, tune down to 5. Decision deferred to L4 evidence.
7. **Engagement signal firing** (see §9 W4-C). Same pattern as feedback — disable until W4-C lands?
8. **`score_salience` "User" actor.** Existing ability allows `Actor::User`. W3-A does NOT touch this allowlist. Confirm no path-α regression slips in via service-layer refactor.

---

## §13 Risk register

| Risk                                                                                                                          | Severity | Mitigation                                                                                                                                                       |
|-------------------------------------------------------------------------------------------------------------------------------|----------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Hidden-ability leakage — `factors` array slips into `SuggestedNextStepItem` via refactor                                       | HIGH     | Privacy sweep test (AC #4); codex security panel reviews the projection function                                                                                  |
| W4-A feedback ability gap (see §9) leaves W3-A shipping non-functional affordances                                             | MED      | Option A (disabled+tooltip) gives honest UI posture; doc'd in PR body                                                                                             |
| Parallel-wave migration slot collision                                                                                         | LOW      | W3 reserves none; W4 lanes claim v274+ per memory rule on parallel-wave slot reservations                                                                          |
| Trust-band rendering drift between Tauri and WP surfaces                                                                       | MED      | Use `ClaimReceiptSnapshot.trust` from Shared Receipt DTO — single source of truth per `v1.4.6-waves.md:543`                                                       |
| Visual parity gap — block ships before reference HTML lands → reviewers can't verify                                           | HIGH     | Reference HTML is an AC (#10); BLOCKS L2                                                                                                                          |
| Block performance — ability call per-page-load adds latency                                                                    | LOW      | Read-only ability over already-stored substrate; per-call cost is single SQL + projection; benchmark in L4 if regression flagged                                  |
| Editor / view scripts not needed in W3-A but added speculatively                                                               | LOW      | Memory: "no half-finished implementations" — W3-A view.js exists only if §4.5 click handler decision lands on Option B; otherwise affordance markup is server-only |

---

## §14 Definition of Done

Per CLAUDE.md DoD section:

1. Each AC in §5 validated against real seeded data (W2 surfacing decisions for the four entity classes; per-subject and global feed)
2. End-to-end flow: WP block invokes ability → projection over substrate → renders trust band + why-this-now + receipt provenance — verified in local WP install
3. No stubs, TODOs, or "Phase 2" deferrals — except the feedback-affordance disabled state (which is the explicit Option A pattern, documented and bounded)
4. `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit && phpunit (wp tests)` all green
5. L2-status declared in code commits (memory rule)
6. L4 evidence captured before L2 dispatch (`v1.4.6-waves.md:440`)
7. K-out: any new class-pattern findings filed via `/ce-compound mode:headless` at retro close

---

## §15 Reviewer routing summary

| Reviewer                                  | What they own                                                                                  |
|-------------------------------------------|------------------------------------------------------------------------------------------------|
| `/codex challenge`                        | Adversarial — try to break the producer-boundary invariant and the hidden-ability privacy contract |
| `/plan-eng-review`                        | Architecture, ability shape, projection algorithm, test plan, CI gate inputs                   |
| `/plan-design-review`                     | Reference HTML strategy, token reuse, parity matrix, affordance pattern                        |
| `/plan-devex-review`                      | New ability ergonomics, schema-version policy, MCP-future-compat, claim_receipt reuse           |
| `ce-learnings-researcher` (parallel K-in) | Confirm no `docs/solutions/` or `.docs/decisions/` precedent missed                            |
| WP-skill-grounded reviewer (5th panel)    | block.json shape, `usesContext` correctness, render.php server-render correctness, style.css enqueue path |

Unanimous APPROVE required. Path-α findings (theoretical hardening) routed to Codebase Maintenance project per memory rule.
