---
ticket: DOS-332
title: "v1.4.6 W4-A — Recommendation feedback loop"
parent_plan: ../v1.4.6-waves.md
fork_sha: 2a6ec716
branch: codex/v1.4.6-w4a-recommendation-feedback
status: L0 — plan hardening, cycle 0 draft
authors: James Giroux, Claude
related:
  - ADR-0123 (Typed Claim Feedback Semantics — 10 variants, V1.1)
  - ADR-0102 (Abilities as Runtime Contract)
  - ADR-0105 (Provenance as First-Class Output)
  - ADR-0125 (Claim Anatomy / Sensitivity / TypeRegistry)
  - DOS-332 (this ticket)
  - DOS-298 (W3-A — Suggested Next Steps block contract, merged)
  - DOS-331 (W2-A — Surfacing policy, merged; owns cooldown state)
  - DOS-329 (W1-A — RecommendationClaim contract, merged; owns FeedbackState shape)
---

# v1.4.6 W4-A — Recommendation feedback loop (DOS-332)

## Cycle 1 amendments (2026-05-27) — 4-reviewer panel resolutions

Cycle 0 panel: adversarial BLOCKED (3 BLOCKED + 5 NEEDS_REVISION + 2 NIT); feasibility CYCLE_2 (1 BLOCKED + 5 NEEDS_REVISION); scope-guardian CONDITIONAL APPROVE (1 BLOCKED + 3 TRIMs); K-in NO REINVENTION (but with substrate-side answers that resolve most BLOCKED items by pointing at what exists).

**The class pattern across reviewers:** the cycle-0 packet was authored without enough grounding in the actual substrate. It invented APIs (`bump_cooldown`), missed ADR-0123 variants (`NotRelevantHere`, `NeedsNuance`, `SurfaceInappropriate`), and proposed a new signal where one already exists. Cycle 1 reframes the packet around primitives that actually exist — verified against code at:

- `src-tauri/src/services/recommendations/surfacing.rs:33` (`SURFACING_DECISION_SIGNAL` is a string const, not an enum variant)
- `src-tauri/src/services/recommendations/surfacing.rs:120,136` (`feedback_suppression_days: i64` default 14; cooldown is read-derived)
- `src-tauri/src/services/recommendations/surfacing.rs:724` (`latest_feedback_suppression` reads `claim_feedback` rows)
- `src-tauri/src/services/claims.rs:5488-5527` (`validate_feedback_actor` admits ONLY `User`; SurfaceClient/Agent/System all rejected by `actor_class_for_actor`)
- `src-tauri/src/services/claims.rs:9127` (`claim_feedback_recorded` signal already emits from `record_claim_feedback`)
- `src-tauri/abilities-runtime/src/abilities/feedback.rs:39` (`FeedbackAction` enum has all 10 ADR-0123 variants including `NotRelevantHere`, `NeedsNuance`, `SurfaceInappropriate`)
- `src-tauri/src/migrations/269_recommendation_claim_metadata_indexes.sql:9-18` (`feedback_state` lives in `intelligence_claims.metadata_json` as JSON — indexed via `json_extract`, NOT a column)

### B1. The mapping table reframed — every variant writes via `record_claim_feedback` (supersedes §1)

The cycle-0 mapping was wrong on three rows and confused on three more. Cycle 1 uses ADR-0123's existing variant catalog faithfully. **Every `RecommendationFeedbackDecision` variant writes to `claim_feedback` with the appropriate `FeedbackAction`.** Cooldown emerges automatically from the read side (`latest_feedback_suppression`); there is no separate cooldown table, no separate cooldown writer.

| `RecommendationFeedbackDecision` | `FeedbackAction` | Truth/Trust effect | Surfacing effect | Rationale |
|---|---|---|---|---|
| `Accept { at }` | `ConfirmCurrent` | +α on source / agent | None | Standard reinforcement (ADR-0123 §149 row #1) |
| `Dismiss { reason: NotRelevant }` | `NotRelevantHere` | No trust delta | 14-day cooldown via `feedback_suppression_days` | "True but not relevant here" — ADR-0123 §149 explicitly for this case. Resolves adv F2. |
| `Dismiss { reason: AlreadyKnew }` | `NotRelevantHere` | No trust delta | 14-day cooldown | Not disputing truth, just not useful surfacing. Same as NotRelevant. |
| `Dismiss { reason: WrongSubject }` | `WrongSubject` | -0.3 subject_evidence on linker; source untouched | Tombstones the claim on the asserted subject (per-subject) | Direct ADR-0123 mapping |
| `Dismiss { reason: Other(BoundedNote) }` | `NotRelevantHere` with `note` field populated | No trust delta; note captured | 14-day cooldown | Conservative default — user provided free text but not a structured judgment; treat as "not relevant" + log the note via ADR-0123's typed `note: Option<String>` column (NOT `payload_json`). Resolves adv F3. |
| `NotUseful { at }` | `SurfaceInappropriate` | No trust delta | 14-day cooldown + flags for review | ADR-0123 §149 overflow set — "this surface placement is wrong"; ranking-only |
| `TooNoisy { at }` | `SurfaceInappropriate` | No trust delta | 14-day cooldown (same as NotUseful at substrate level; intensity is a UX framing, not a substrate distinction) | Per K-in: K-in could not find a §149 variant that distinguishes "too noisy" from "surface inappropriate." Both map to the same FeedbackAction. Cooldown bump is uniform; future cross-claim aggregation (ADR-0123 §207 deferred to v1.5.0+) handles "user often marks X as noisy" learning. |
| `Convert { at, into: Action(action_id) }` | `ConfirmCurrent` | +α on source / agent | None | Reinforcement + open-loop attachment. Open-loop attachment is a SEPARATE service call (`services::actions::attach_from_recommendation` or equivalent); not transactional with the claim_feedback write because attachment may invoke external side effects (calendar / message scheduling). **Cycle 1 explicit:** the ability returns success after `claim_feedback` write; open-loop attachment is at-least-once via a follow-up service call within the same ability body. If attachment fails, the feedback row stays (truth-feedback is authoritative); the attachment is retried via existing action retry infrastructure. Resolves adv F4. |
| `Convert { at, into: ClaimCorrection(claim_id) }` | `NeedsNuance` with `corrected_text` payload | Refinement path per ADR-0123 §149 row #7 | Cooldown on original subject | ADR-0123 `NeedsNuance` is **specifically named** for user-authored corrections via text-overlap heuristic. K-in cited ADR-0123 §154 structured-only rules as routing structured corrections elsewhere — but §154 covers structured corrections IN the feedback API. Our `ClaimCorrection(claim_id)` references an EXISTING claim that already went through propose/commit. So the original recommendation's correctness is refined; `NeedsNuance` with the corrected claim_id as evidence is the cleanest mapping. Resolves adv F8. |
| `Convert { at, into: ReviewQueue(queue_item_id) }` | None (no claim_feedback row) | No trust delta | No cooldown | DOS-336 review queue routes independently; this variant emits the queue item id only. W4-A doesn't write claim_feedback here. |

**Q1/Q2/Q3 from cycle-0 §12 are RESOLVED by this table** (per scope-guardian TRIM):
- Q1: `Dismiss { Other }` → `NotRelevantHere` with typed `note`
- Q2: `NotUseful` vs `TooNoisy` cooldown magnitude → same at substrate; UX distinction is surface-side
- Q3: `Convert { ClaimCorrection }` → `NeedsNuance` (substrate keeps the original claim refining-path semantic)

### B2. Cooldown is read-derived — drop the `bump_cooldown` API (supersedes §1, §3.4, §11)

Cooldown is NOT a separate write API. `services::recommendations::surfacing::latest_feedback_suppression` reads `claim_feedback` rows within the policy's `feedback_suppression_days` window (default 14) and surfaces them through `feedback_suppressed_recently: bool` in the surfacing state. **A feedback write IS the cooldown bump.**

This deletes the `bump_cooldown` references throughout cycle-0 §1 / §3.4 / §11. The W4-A scope shrinks: no cross-lane coordination with W2-A, no scope creep into surfacing.rs, no parallel cooldown table. Resolves adv F1 + feas #2.

### B3. Signal: reuse `claim_feedback_recorded`, do not declare new (supersedes §3.1, §3.4 step 5, §4 AC #8)

`record_claim_feedback` already emits the `claim_feedback_recorded` signal (string const at `services::claims:9127`). All recommendation feedback paths flow through `record_claim_feedback`, so this signal fires automatically for W4-A writes.

The cycle-0 packet's new `RecommendationFeedbackRecorded` SignalType variant is **dropped**. Resolves scope F4 (W0 ownership concern — by dropping the new variant, W0 ownership of `signals/policy_registry.rs` is preserved) and feas #6.

If W4-B (deviation) or W4-C (engagement) need finer-grained discrimination, they consume `claim_feedback_recorded` with payload filtering — the existing signal carries `feedback_id`, `claim_id`, `feedback_type` (which is the `FeedbackAction`). For the `NeedsNuance` and overflow-set variants, downstream lanes can filter by `feedback_type`. No new signal needed.

### B4. Category is `Maintenance`, not `Write` (supersedes §3.1)

Per ADR-0102 §82, ability categories are call-graph-derived (Read / Transform / Publish / Maintenance), not author-declared. `submit_recommendation_feedback` mutates internal state through `services::claims::record_claim_feedback` (which itself is the canonical maintenance path for `intelligence_claims` mutations). The ability is **Maintenance**.

ADR-0103 maintenance-ability constraints apply: `may_publish = false`, `client_side_executable = false`, no MCP exposure for maintenance abilities by default (`mcp_exposure = None`). Updated:

- **`category`:** `Maintenance` (not `Write`)
- **`required_scopes`:** `["maintenance.recommendations.feedback"]` (replaces the cycle-0 `write.recommendations` since maintenance scopes follow a different naming convention; verify at L1 grep against existing maintenance scope strings)
- **`mutates`:** `["intelligence_claims", "claim_feedback"]` declared at the macro

This is the **first Maintenance ability under `recommendations/`** (cycle-0 was framed as the first Write; that framing was wrong). Inventory pattern: existing Maintenance abilities elsewhere in the codebase establish the pattern; W4-A follows.

### B5. Actor normalization — SurfaceClient → "user" at the ability boundary (supersedes §3.1, §3.4)

`validate_feedback_actor` (`services::claims:5507`) admits only `User` (`"user"` or `"human"` strings via `actor_class_for_actor`). SurfaceClient string `"surface_client"` is unmapped and would hard-fail with `InvalidFeedback`.

**Resolution:** the ability normalizes `ctx.actor()` to `"user"` before calling `record_claim_feedback`. The actor-class check inside `record_claim_feedback` is for the CONTENT of the feedback (it IS a user-class feedback regardless of which surface posted it). The original Actor identity (`SurfaceClient { instance, scopes }`) is preserved in `RecommendationFeedbackContext` (existing field) and in the ability's provenance envelope, so the audit trail records the surface that posted.

This is a 2-line normalization at the ability body, not an extension of `actor_class_for_actor`. Resolves feas #1.

### B6. `feedback_state` mutation is JSON-patch, not column UPDATE (supersedes §3.4 step 4)

`RecommendationClaim.feedback_state` lives in `intelligence_claims.metadata_json` as a JSON field, indexed via `json_extract(metadata_json, '$.recommendation.feedbackState…')` per migration `269_recommendation_claim_metadata_indexes.sql`. The mutation path uses SQLite's `json_set`:

```sql
UPDATE intelligence_claims
SET metadata_json = json_set(metadata_json, '$.recommendation.feedbackState', ?, '$.recommendation.conversionState', ?)
WHERE id = ?
  AND json_extract(metadata_json, '$.recommendation.feedbackState') LIKE '%"pending"%'
```

The `WHERE … LIKE '%"pending"%'` clause is the **atomic compare-and-set** for idempotency (resolves adv F6): if the row's feedback_state is already non-Pending, the UPDATE affects 0 rows, the ability returns `EffectKind::NoMutation`. SQLite's single-writer guarantee (per feas #4) prevents the read-then-write race. The packet cites SQLite single-writer rather than implying row-level locking.

### B7. Output type trim — `EffectKind` replaces `DownstreamEffect` (supersedes §3.3)

Per scope-guardian TRIM: the cycle-0 `DownstreamEffect` enum carried inner fields (`feedback_id`, `new_cooldown_until`, etc.) for a confirmation-chip use case in a surface that's parked. Trim to a leaner discriminator:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    ClaimFeedbackRecorded,
    NoMutation,
}
```

`NoMutation` is the idempotent-reject case (already-decided per B6). When a surface ships and wants confirmation-chip metadata, that ability evolution is additive (add `feedback_id` field; bump schema_version).

The cycle-0 `BothApplied` and `SurfacingCooldownBumped` variants are gone because B2 eliminated the separate cooldown path. Every successful write produces `ClaimFeedbackRecorded`; cooldown is read-side.

`Convert { Action }` still attaches the open-loop; its success/failure does NOT show up in `EffectKind` — instead, the response carries an `Option<ActionAttachment>` field for the surface to render. Failure of the open-loop attach does not roll back the feedback (per B1 Convert row + adv F4).

### B8. Output wrapped in `AbilityOutput<T>` for provenance (supersedes §3.3)

Per K-in (ADR-0105 §8 "lives-once" invariant), the response shape is:

```rust
pub type SubmitRecommendationFeedbackOutput = AbilityOutput<SubmitRecommendationFeedbackResponse>;
```

Provenance envelope lives once on the `AbilityOutput`; the response struct itself carries no envelope copy. Standard `ProvenanceBuilder` pattern used by all v1.4.6 abilities.

### B9. WP REST Write-path verified (resolves feas #8 + adv F5)

Per feas grep: `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:85` exposes `invoke_ability(name, payload, scope_set)` category-agnostic. No category-specific branching in the transport. This is the **first Maintenance ability** to traverse it (cycle-0 mis-framed as Write), but the transport admits all categories. No new transport work.

L1 verification step: confirm `tools/dailyos-abilities.json` regen handles `Maintenance` category as passthrough (per feas #9 — existing inventory has Read/Transform/Maintenance categories already, so this is exercised).

### B10. Cycle 0 §12 open questions — all resolved

- Q1 (`Dismiss { Other }` mapping) → B1 row: `NotRelevantHere` with typed `note`
- Q2 (NotUseful vs TooNoisy) → B1 rows: same FeedbackAction (`SurfaceInappropriate`); UX distinction is surface-side
- Q3 (Convert { ClaimCorrection } target) → B1 row: `NeedsNuance` with corrected_text payload
- Q4 (Signal payload shape) → B3: dropped new signal; consume existing `claim_feedback_recorded`
- Q5 (WP REST endpoint shape) → B9: existing transport admits all categories

### B11. CI gate inputs add per-Actor dry-run row (supersedes §7)

Per scope-guardian TRIM: per-Actor dry-run test enumerated in §5 channel #6 and §6 unit tests but missing from §7 CI gate table. Added:

| Artifact | Notes |
|---|---|
| Per-Actor dry-run test | `[User, SurfaceClient]` admit; `[System, Agent, Admin, McpClient]` deny with `Capability` error. Lives in `abilities-runtime/src/abilities/recommendations/mod.rs::tests`. CI-enforced gate, not advisory. |

### B12. DoD additions (supersedes §14)

Three additions per adv F9 + F10:

- **L4 filter-flip gate.** The W3-A WP block's `dailyos_suggested_next_steps_feedback_enabled` filter flip from `false` to `true` is **NOT** an automatic post-merge step. It is an L4-gated change requiring hands-on QA of the full WP block → REST → ability → `record_claim_feedback` → claim_feedback row → surfacing.rs read pickup vertical. The flip ships separately from W4-A merge, gated by L4 evidence + DOS-298 / DOS-333 readiness.
- **Privacy non-leak documented.** `EffectKind::ClaimFeedbackRecorded` carries no subject identity; surface that submitted the feedback already knows its own claim's subject. No new privacy surface introduced. Documented in §3.3.
- **Auth overhaul note.** ADR-0111 §193 nonce requirement for write events is currently being stripped for local same-user contexts per the 2026-05-21 auth overhaul (per memory). W4-A does NOT add nonce machinery; it relies on the post-overhaul actor-allowlist model. If the overhaul lands differently than expected, W4-A's transport gate at L1 grep adds nonce verification.

### B13. Scope shrinks net

After cycle 1: NO new tables, NO new SignalType variants, NO new cooldown API, NO new scope (just a maintenance-scope rename), NO new column (json_set on metadata_json). The ability + service-function fill (`feedback.rs`) is the entire scope.

This is appropriately smaller than W3-A. Per scope-guardian: "If W4-A's packet bloats toward W3-A scale, that's a signal." Cycle 1 trims it back to substrate-extension.

### Cycle 1 status

- B1–B12 amendments apply; B13 trim confirmed
- Original §1, §3.1, §3.3, §3.4, §4, §7, §12, §14 sections are SUPERSEDED inline by cycle 1 (the inline-sweep pattern from W3-A's cycle 3); a future cycle-2 reviewer reading any single section should see consistent state without needing to cross-reference cycle 1 amendments

**Ready for cycle 2 panel re-dispatch.** Recommended subset: adversarial (verify F1/F2/F3/F8 closed); feasibility (verify B6 json_set + B5 actor normalization + B9 transport); scope-guardian (verify B3/B4 dropped scope; B11 added test row). Skip K-in (no scope changes need re-validation).

---

## §0 Frame

W4-A is **substrate-only**. Per the UI Surface Deferral Amendment (2026-05-27), v1.4.6 continues with backend wiring while WP and Tauri surfaces wait for production readiness + redesign respectively. W4-A delivers the feedback ability + service path so that *when* a surface ships its affordance, the producer is ready.

Scope per `v1.4.6-waves.md:1005-1022`:

1. Fill the placeholder at `src-tauri/src/services/recommendations/feedback.rs` (today: 7-line docblock — see read at L0).
2. Author the recommendation-feedback wrapper API: `record_recommendation_feedback(claim_id, decision, context) -> FeedbackState`.
3. Map each `RecommendationFeedbackDecision` variant (Accept / Dismiss{reason} / NotUseful / TooNoisy / Convert) onto:
   - the existing `FeedbackAction` enum + `services::claims::record_claim_feedback` write path (for truth-feedback semantics) OR
   - the surfacing cooldown state (for ranking-only feedback, never trust)
4. Ship the typed `FeedbackState::Pending → FeedbackState::Decided(...)` transition on the `RecommendationClaim`.
5. Tests verify each variant produces the documented ADR-0123 action and the right downstream effect (trust vs surfacing vs ranking).

**No new tables.** `claim_feedback` is the existing storage. ADR-0123's 10-variant enum is fixed; W4-A consumes it.

**No new ADR.** This is a wrapper around existing primitives.

**No UI dependency.** W3-A's WP block has affordance markup but disabled handler (Option A). When this lane ships, the block's `dailyos_suggested_next_steps_feedback_enabled` filter flips to true. DOS-802's Tauri surface follows the same pattern when it eventually lands.

**Reviewer routing (per CLAUDE.md L0 default + W4-A wave-plan W4-A-specific):**

- `/codex challenge` — adversarial planning reviewer (parallel)
- `ce-feasibility-reviewer` — substrate alignment (claim_feedback shape, FeedbackAction surface, cooldown state)
- `ce-scope-guardian-reviewer` — confirm no parallel tables, no new ADR, no UI scope creep
- `ce-learnings-researcher` — K-in mandatory parallel (look for prior feedback-mapping work, ADR-0123 amendments, prior feedback solution memories)

No `ce-design-lens-reviewer` — substrate-only. No `ce-security-lens-reviewer` — W4-A doesn't touch actor exposure or telemetry (W4-C does; this lane doesn't).

---

## §1 The mapping (locked at cycle 1; see B1)

**Every `RecommendationFeedbackDecision` variant writes to `claim_feedback` via `record_claim_feedback` with the appropriate ADR-0123 `FeedbackAction`.** Cooldown emerges automatically from the read side (`services::recommendations::surfacing::latest_feedback_suppression`); there is no separate cooldown table or writer.

| `RecommendationFeedbackDecision` | `FeedbackAction` | Truth/Trust effect | Surfacing effect (read-derived) |
|---|---|---|---|
| `Accept { at }` | `ConfirmCurrent` | +α on source / agent | None |
| `Dismiss { reason: NotRelevant }` | `NotRelevantHere` | No trust delta | 14-day cooldown via `feedback_suppression_days` |
| `Dismiss { reason: AlreadyKnew }` | `NotRelevantHere` | No trust delta | 14-day cooldown |
| `Dismiss { reason: WrongSubject }` | `WrongSubject` | -0.3 subject_evidence on linker; source untouched | Tombstones the claim on the asserted subject (per-subject) |
| `Dismiss { reason: Other(BoundedNote) }` | `NotRelevantHere` with `note` field populated | No trust delta; note captured in ADR-0123 typed `note: Option<String>` column | 14-day cooldown |
| `NotUseful { at }` | `SurfaceInappropriate` | No trust delta | 14-day cooldown |
| `TooNoisy { at }` | `SurfaceInappropriate` | No trust delta | 14-day cooldown (same as NotUseful at substrate level; intensity is UX framing, not substrate distinction) |
| `Convert { at, into: Action(action_id) }` | `ConfirmCurrent` | +α on source / agent | None. **Open-loop attachment is a separate at-least-once call** (`services::actions::attach_from_recommendation` or equivalent); not transactional with the feedback write because attachment may invoke external side effects (calendar / message scheduling). If attachment fails, the feedback row stays (truth-feedback is authoritative); attachment retries via existing action retry infrastructure. |
| `Convert { at, into: ClaimCorrection(claim_id) }` | `NeedsNuance` with `corrected_text` payload referencing the corrected claim_id | Refinement path per ADR-0123 §149 row #7 | Cooldown on original subject |
| `Convert { at, into: ReviewQueue(queue_item_id) }` | None (no claim_feedback row) | No trust delta | None — DOS-336 review queue routes independently |

**Rationale citations:**

- `NotRelevantHere` (ADR-0123 §149) is the explicit variant for "true but not relevant here — no trust delta." Used for `Dismiss{NotRelevant|AlreadyKnew|Other}`.
- `SurfaceInappropriate` (ADR-0123 §149 overflow set) covers "this surface placement is wrong — ranking-only." Used for `NotUseful` and `TooNoisy`.
- `WrongSubject` (ADR-0123 §1) tombstones at the asserted subject; linker takes the -0.3 hit; source untouched. Used only for explicit `Dismiss{WrongSubject}`.
- `NeedsNuance` (ADR-0123 §149 row #7) is specifically named for user-authored corrections via text-overlap heuristic. Used for `Convert{ClaimCorrection}`.
- `ConfirmCurrent` (ADR-0123 §149 row #1) is standard reinforcement. Used for `Accept` and `Convert{Action}`.

**Cooldown:** `surfacing.rs::latest_feedback_suppression` reads `claim_feedback` rows within `feedback_suppression_days` (default 14, per `SurfacingPolicy` at `surfacing.rs:120,136`). Every feedback write IS the cooldown bump; no separate writer needed.

**FeedbackState transition:** every successful variant transitions `Pending → Decided(<the variant>)` on the `RecommendationClaim`'s `metadata_json` JSON field via `json_set` (NOT a column UPDATE — the field is indexed via `json_extract` per migration `269_recommendation_claim_metadata_indexes.sql`). The mutation is wrapped in an atomic compare-and-set: `UPDATE … WHERE json_extract(metadata_json, '$.recommendation.feedbackState') LIKE '%"pending"%'`. SQLite single-writer guarantees the race-free path (see B6).

---

## §2 Scope + producer authority

### In scope

| File / surface | Owner |
|---|---|
| `src-tauri/src/services/recommendations/feedback.rs` (fill 7-line placeholder) | W4-A |
| `src-tauri/abilities-runtime/src/abilities/recommendations/` — new `submit_recommendation_feedback` ability | W4-A |
| `src-tauri/abilities-runtime/src/abilities/recommendations/contracts.rs` — additive input/output types for the new ability | W4-A |
| `src-tauri/abilities-runtime/src/abilities/recommendations/mod.rs` — register ability | W4-A |
| `src-tauri/src/services/context.rs` — wire feedback handle | W4-A |
| `tools/dailyos-abilities.json` — regen | W4-A |
| Rust unit tests covering all 10 mapping rows above (parametric) | W4-A |
| Integration test: full Pending → Decided cycle with seeded `RecommendationClaim` | W4-A |

### Out of scope

- WP block UI changes (block already has affordance markup gated by `dailyos_suggested_next_steps_feedback_enabled`; flip after W4-A merges OR in a follow-up depending on WP surface readiness — see UI Surface Deferral)
- Tauri React UI (DOS-802 parked)
- New tables / migrations (use `claim_feedback` + W2-A cooldown state)
- New ADR-0123 variants (consume the 10 existing)
- Engagement telemetry path (W4-C lane)
- Deviation detection (W4-B lane)
- Salience re-compute timing changes (W1-B owns; W4-A emits the feedback that re-compute consumes on next cycle)

### Producer authority restatement

Wave plan §line 1015-1020:
- Claim model: feedback writes through service path (existing `record_claim_feedback`)
- Provenance: user feedback affects future salience factor weights (UserFit factor — W1-B re-compute consumes)
- Signals: emit existing claim-feedback signals; no new signal types
- Runtime: wired from W3-A action handlers (when UI flips on)
- Feedback loop: this IS the feedback loop primitive

---

## §3 Ability contract

### §3.1 Identity

- **Name:** `submit_recommendation_feedback`
- **Category:** `Maintenance` — per ADR-0102 §82 the category is call-graph-derived. Mutates internal state through `services::claims::record_claim_feedback` (canonical maintenance path for `intelligence_claims` mutations). ADR-0103 maintenance-ability constraints apply.
- **Allowed actors:** `[User, SurfaceClient]` — the user OR a surface acting on the user's explicit click. NOT `System` (no automatic feedback), NOT `Agent` / `Admin`, NOT `McpClient`. Note: per B5, the ability **normalizes `ctx.actor()` to `"user"`** before calling `record_claim_feedback` because `validate_feedback_actor` (`claims.rs:5507`) admits only `User`. The original SurfaceClient identity is preserved in the provenance envelope.
- **`required_scopes`:** `["maintenance.recommendations.feedback"]` (maintenance naming convention; verify at L1 grep against existing maintenance scope strings)
- **`may_publish`:** false (per ADR-0103 maintenance default)
- **`mcp_exposure`:** `None` (per ADR-0103 maintenance default)
- **`client_side_executable`:** false (per ADR-0103)
- **`mutates`:** `["intelligence_claims", "claim_feedback"]` declared at the `#[ability(...)]` macro
- **`composes`:** `[]` (does NOT compose `claim_receipt` — idempotent compare-and-set on `feedback_state` happens in the UPDATE itself, no separate read needed)
- **`signal_policy.emits_on_output_change`:** `[]` — the existing `claim_feedback_recorded` signal fires automatically from `record_claim_feedback` (`claims.rs:9127`); W4-A does not declare a new signal (B3).
- **Schema version:** 1

### §3.2 Input

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRecommendationFeedbackInput {
    pub schema_version: u32, // pinned to 1 per A14 schema-version-cliff policy
    pub claim_id: ClaimId,
    pub decision: RecommendationFeedbackDecision, // existing enum from contracts.rs
    pub context: RecommendationFeedbackContext,   // existing struct (surface + invocation_id)
}
```

`RecommendationFeedbackDecision` and `RecommendationFeedbackContext` already exist in `src-tauri/src/services/recommendations/contracts.rs` (W1-A landed them). No new types required.

### §3.3 Output

The ability returns `AbilityOutput<SubmitRecommendationFeedbackResponse>` per ADR-0105 §8 "lives-once" invariant. The provenance envelope rides on the wrapper; the inner response carries no envelope copy.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRecommendationFeedbackResponse {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub feedback_state: FeedbackState,                // always Decided(<variant>) after success; Pending on NoMutation
    pub conversion_state: ConversionState,             // updated if Convert variant
    pub effect_kind: EffectKind,
    pub action_attachment: Option<ActionAttachment>,   // only Some(...) for Convert{Action}; carries the open-loop action_id
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    ClaimFeedbackRecorded,
    NoMutation, // idempotent reject — claim's feedback_state was already non-Pending at write time
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionAttachment {
    pub action_id: String,
    pub attached_at: DateTime<Utc>,
    pub attach_status: AttachStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AttachStatus {
    /// Open-loop attached successfully within this ability call
    Attached,
    /// Feedback recorded; open-loop attach queued for at-least-once retry (per B1 Convert{Action} row)
    Queued,
}
```

**Privacy note:** `EffectKind` carries no subject identity. The caller (the surface that posted the feedback) already knows its own claim's subject and decision. No new privacy surface introduced (resolves adv F10).

**Why `EffectKind` not the full `DownstreamEffect` from cycle 0:** per scope-guardian TRIM (B7) — the cycle-0 enum carried fields (`feedback_id`, `new_cooldown_until`, `action_signature`, etc.) for a confirmation-chip surface that's parked. When a surface lights up and needs confirmation metadata, schema-version bump adds fields additively.

### §3.4 Service-side flow

`services::recommendations::feedback::record_recommendation_feedback`:

1. **Resolve claim.** Load the `RecommendationClaim` by `claim_id` via `load_claim_by_id`. Reject with `ClaimError::UnknownClaimId` if not found, or `ClaimError::UnsupportedClaimType` if not a recommendation claim.
2. **Normalize actor.** `ctx.actor()` resolves to either `Actor::User` or `Actor::SurfaceClient { ... }`. The downstream call to `record_claim_feedback` requires the `actor` string to map to `ClaimActorClass::User` via `actor_class_for_actor` (`claims.rs:5488-5504`) — which admits only `"user"` or `"human"`. The ability normalizes by passing `"user"` regardless of which actor variant invoked. The original Actor identity (with `SurfaceClient.instance` / `SurfaceClient.scopes` if applicable) is captured in the provenance envelope at the ability boundary, so audit trail is complete.
3. **Map `decision` variant per §1 table.** Constructs a `ClaimFeedbackInput` with the mapped `FeedbackAction` and an appropriate `note` (for `Dismiss{Other(BoundedNote)}`) or `payload_json` (for `Convert{ClaimCorrection}` carrying `corrected_text` per ADR-0123 §149 row #7).
4. **Idempotent compare-and-set on `feedback_state`.** SQLite atomic UPDATE within `with_claim_transaction`:
   ```sql
   UPDATE intelligence_claims
   SET metadata_json = json_set(metadata_json,
       '$.recommendation.feedbackState', ?,
       '$.recommendation.conversionState', ?)
   WHERE id = ?
     AND json_extract(metadata_json, '$.recommendation.feedbackState') LIKE '%"pending"%'
   ```
   If `rows_affected == 0`, return `Ok(EffectKind::NoMutation)` with the existing state — the claim was already decided. SQLite single-writer prevents the read-then-write race (per feas #4); the WHERE-clause check is the atomic guard. **Resolves adv F6.**
5. **Call `record_claim_feedback`.** With the constructed `ClaimFeedbackInput` and the normalized `"user"` actor string. This:
   - Inserts the `claim_feedback` row (UUID generated internally per `claims.rs:7156`)
   - Validates the payload per `validate_feedback_payload` (L1 verifies the `note` + `corrected_text` keys are admitted by the validator for the `NotRelevantHere` / `NeedsNuance` / `SurfaceInappropriate` actions; if not, L1 amendment to the validator's allowlist)
   - Transitions verification_state per ADR-0123 dispatch table
   - Emits `claim_feedback_recorded` signal automatically (no W4-A action needed — per B3)
6. **Convert{Action} side effect.** If the decision is `Convert{Action(action_id)}`, AFTER the feedback row is inserted, invoke `services::actions::attach_from_recommendation(action_id, recommendation_claim_id)` (or the existing analog — L1 grep confirms the function name). If attachment fails, the feedback row stays; the response's `action_attachment.attach_status = Queued` signals at-least-once retry. **Resolves adv F4 atomicity-honesty.**
7. **Convert{ReviewQueue} side effect.** If the decision is `Convert{ReviewQueue(queue_item_id)}`, skip the `record_claim_feedback` call entirely (per §1 table); just update `conversion_state` via `json_set` on `metadata_json` and return. DOS-336's review-queue lifecycle owns the conversion semantics.
8. **Return `AbilityOutput<SubmitRecommendationFeedbackResponse>`.** Wrap in provenance via `ProvenanceBuilder` per ADR-0105.

All steps 4–7 are within `with_claim_transaction` (which provides SQL transaction boundaries via SQLite's serializable isolation). External side effects (action attachment in step 6) cannot be rolled back — that's the at-least-once contract documented in `AttachStatus::Queued`.

---

## §4 Acceptance criteria

1. **Ability ships and is registered.** `submit_recommendation_feedback` in `tools/dailyos-abilities.json` (regen); `AbilityRegistry` exposes to `[User, SurfaceClient]` with `category = Maintenance`. `cargo test recommendations::submit_recommendation_feedback` passes.
2. **Mapping is deterministic.** Parametric test enumerates all 10 mapping rows in §1; asserts the documented `(FeedbackAction, EffectKind)` pair for each. Each row's claim_feedback row inspection confirms the typed `note` or `payload_json` shape.
3. **Idempotent.** Re-submitting feedback for an already-`Decided` claim returns `EffectKind::NoMutation` and does NOT mutate `claim_feedback` (the atomic compare-and-set UPDATE affects 0 rows; service short-circuits per §3.4 step 4).
4. **Atomicity via SQLite single-writer.** The `metadata_json` json_set UPDATE + the `record_claim_feedback` INSERT are within `with_claim_transaction`. SQLite's serializable isolation prevents partial observation. External side effects (Convert{Action} attachment) are explicitly at-least-once per `AttachStatus::Queued`.
5. **No new tables.** Migrations diff is empty. Uses existing `claim_feedback` rows; cooldown is read-derived from `latest_feedback_suppression` (per B2).
6. **No new ADR-0123 variants.** Maps to the existing 10 `FeedbackAction` variants from `abilities-runtime/src/abilities/feedback.rs:39`.
7. **Privacy.** `Dismiss { reason: Other(BoundedNote) }` enforces 200-char cap via `BoundedNote::try_from`. The note is stored in `claim_feedback`'s typed `note: Option<String>` column per ADR-0123 §2 (NOT `payload_json` — resolves adv F3). L1 verifies `validate_feedback_payload` admits the `note` key for `NotRelevantHere`; if not, L1 extends the validator.
8. **Signal: reuse `claim_feedback_recorded`.** The existing signal fires automatically from `record_claim_feedback` (`claims.rs:9127`). W4-A does NOT declare a new SignalType. Downstream W4-B/C consume the existing signal and filter by `feedback_type` (per B3).
9. **Actor normalization at the ability boundary.** Per B5: `ctx.actor()` is normalized to `"user"` before calling `record_claim_feedback` so `validate_feedback_actor` admits. Original Actor identity preserved in the provenance envelope.
10. **`feedback_state` is JSON, not a column.** Mutation via `json_set` on `metadata_json` (per migration `269_recommendation_claim_metadata_indexes.sql`). Per B6.
11. **W3-A filter-flip is L4-gated, not auto-merged.** Per B12: the `dailyos_suggested_next_steps_feedback_enabled` filter flip from `false` to `true` is a separate L4-gated change requiring hands-on QA of the full WP block → REST → ability → record_claim_feedback → surfacing read pickup vertical. The flip does NOT ship with W4-A merge.
12. **First Maintenance ability under `recommendations/`.** Per B4 + feas #8: existing `dailyos-abilities.json` inventory has Read/Transform/Maintenance categories; the Maintenance category for recommendations is new but the transport admits it without changes. L1 confirms regen handles it as passthrough.
13. **Cycle hygiene.** `cargo clippy -- -D warnings && cargo test --lib && pnpm tsc --noEmit` clean.

---

## §5 Registry channel enumeration

Per W3-A's A8 pattern (channels enumerated before merge):

1. **Ability registry** — auto via `#[ability(...)]` macro; tested via `tools/dailyos-abilities.json` diff.
2. **Surface scope registry** — new `write.recommendations` scope; verify file path at L1 grep (likely `abilities-runtime/src/abilities/registry.rs`).
3. **WP REST endpoint allowlist** — the new ability must be admitted by `wp/dailyos/includes/transport/class-dailyos-runtime-client.php`'s allowlist (if explicit) or pass through (if dynamic per scope). Verify at L1.
4. **`tools/dailyos-abilities.json`** — regen + commit.
5. **`signals/policy_registry.rs`** — pre-declare `RecommendationFeedbackRecorded` SignalType.
6. **Per-Actor dry-run test** — assert `[User, SurfaceClient]` admit; `[System, Agent, Admin, McpClient]` deny with `Capability` error.

---

## §6 Test plan

### Rust unit

- Mapping table parametric: each `RecommendationFeedbackDecision` variant produces the documented `DownstreamEffect` (10 rows)
- Idempotent re-submission returns `NoMutation`
- `Dismiss { Other }` with note >200 chars rejected at contract level (already enforced by `BoundedNote`)
- Unknown `claim_id` returns `ClaimError::UnknownClaimId`
- Non-recommendation claim_id returns `ClaimError::UnsupportedClaimType`
- Per-Actor allowlist test ([User, SurfaceClient] admit; rest deny)

### Integration (`cargo test --test` recommendations)

- Full Pending → Decided cycle: seed claim, submit Accept, verify `claim_feedback` row + claim's `feedback_state` updated atomically
- Cooldown-only path: submit `NotUseful`, verify no `claim_feedback` row; cooldown bumped in surfacing state
- Both-paths: submit `Convert { ClaimCorrection }`, verify `claim_feedback` `MarkOutdated` row + corrected claim proposal exists
- Signal emission: assert `RecommendationFeedbackRecorded` signal fired with correct claim_id + decision variant

### Substrate-side smoke (deferred to W3-A integration when UI flips)

- WP block view.js → REST endpoint → ability → service path → claim_feedback / surfacing — full vertical test. Deferred per UI Surface Deferral.

---

## §7 CI gate inputs (L1 deliverables per memory rule)

| Artifact | Notes |
|---|---|
| `tools/dailyos-abilities.json` regen | Deterministic per W3-A precedent; first Maintenance ability under `recommendations/` |
| `services::recommendations::feedback` doc comments | Document the §1 mapping table inline so future readers don't need to find this packet |
| Per-Actor dry-run test | Asserts `[User, SurfaceClient]` admit; `[System, Agent, Admin, McpClient]` deny with `Capability` error. Lives in `abilities-runtime/src/abilities/recommendations/mod.rs::tests`. CI-enforced gate. |
| Parametric mapping test | All 10 mapping rows from §1; asserts `(FeedbackAction, EffectKind, note/payload_json shape)` per row. |
| Idempotent compare-and-set test | Re-submit feedback for an already-Decided claim → `EffectKind::NoMutation`; zero new `claim_feedback` rows. |
| `validate_feedback_payload` admits `note` for `NotRelevantHere` | L1 either confirms via grep + test, or extends the validator allowlist for the new key + action pair |
| L2-status in commit messages | `passed` per memory rule |

**No new `signals/policy_registry.rs` row** — per B3, W4-A consumes the existing `claim_feedback_recorded` signal (`claims.rs:9127`); no new SignalType variant declared. W0 ownership preserved.

---

## §8 Migration disposition

**No new migrations.** Empty migrations diff. v273 stays available for W4-B/W4-C if they need storage.

---

## §9 Cross-wave coordination

- **W3-A (merged):** W3-A's WP block has affordance markup gated by `dailyos_suggested_next_steps_feedback_enabled` filter (default false). Post W4-A merge, a one-line follow-up flips this filter. That follow-up is NOT W4-A scope per the UI Surface Deferral — it depends on WP surface readiness.
- **DOS-802 (parked):** Tauri carve-out follows the same pattern when it unblocks.
- **W4-B (DOS-316 deviation):** consumes feedback signal as Novelty factor input. W4-B's L0 reads from `RecommendationFeedbackRecorded` signal declared here.
- **W4-C (DOS-317 engagement):** distinct from feedback (engagement is implicit observation; feedback is explicit click). W4-C imports the same `EngagementSignal` types from W1-A contracts but does not call into W4-A's service.
- **W5 eval (DOS-338):** the eval harness tests "feedback updates ranking on next cycle" — depends on W4-A's signal + W1-B's salience consumer.

---

## §10 Out of scope (restatement)

- WP block UI changes
- Tauri React UI (DOS-802 parked)
- New tables, migrations, or ADR-0123 variants
- Engagement telemetry (W4-C)
- Deviation detection (W4-B)
- Salience re-compute timing (W1-B owns)
- Cross-surface dedup of recommendations (W3-B parked)

---

## §11 K-in citations (preliminary; `ce-learnings-researcher` confirms in parallel)

- ADR-0123 (Typed Claim Feedback Semantics) — the 10-variant enum W4-A maps onto
- ADR-0102 (Abilities as Runtime Contract) — ability registration pattern
- ADR-0105 (Provenance as First-Class Output) — feedback writes carry provenance
- `services::claims::record_claim_feedback` (claims.rs:7128) — existing write path
- `services::recommendations::contracts` — `RecommendationFeedbackDecision`, `RecommendationFeedbackContext`, `BoundedNote` already landed in W1-A
- `abilities-runtime::abilities::feedback` — existing `FeedbackAction` enum + claim-feedback ability (for the bridge keys)
- `services::recommendations::surfacing` — W2-A's cooldown state API (consume from this lane)
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` — class-sweep test pattern (less critical here than W3-A but applicable)

K-in needs to confirm: no prior `submit_recommendation_feedback` solution exists; no parallel feedback-mapping memory documents alternative variant→action mappings.

---

## §12 Open questions

**All cycle-0 open questions resolved by B10.** Remaining open items are L1-resolvable, not L0-blocking:

1. **`validate_feedback_payload` admits `{ note: "..." }` for `NotRelevantHere`?** L1 reads `claims.rs:5525+` (the `validate_feedback_payload` body). If admitted: proceed. If not admitted: either (a) extend the validator allowlist for `NotRelevantHere` + `note` key, or (b) re-map `Dismiss{Other}` to `SurfaceInappropriate` if THAT action admits a note. Decision made at L1 against the live validator; not blocking L0.
2. **`services::actions::attach_from_recommendation`** — the exact function name + signature for the open-loop attachment in `Convert{Action}` step 6. L1 grep confirms the existing function; if no such function exists, L1 either authors one (small scope, matches existing action service patterns) or routes through whatever the current attachment path is. Not blocking L0.

**Resolved at cycle 1:**
- Q1 (cycle 0) `Dismiss { Other }` → `NotRelevantHere` with typed `note` column (B1)
- Q2 (cycle 0) `NotUseful` vs `TooNoisy` → same `SurfaceInappropriate` (B1)
- Q3 (cycle 0) `Convert { ClaimCorrection }` → `NeedsNuance` (B1)
- Q4 (cycle 0) Signal payload → consume existing `claim_feedback_recorded` (B3)
- Q5 (cycle 0) WP REST endpoint → existing transport admits all categories (B9)

---

## §13 Risk register

| Risk | Severity | Mitigation |
|---|---|---|
| Mapping table is wrong (a variant punishes source when it shouldn't) | HIGH | Reviewer panel hits this hardest; parametric test asserts each variant's downstream_effect_kind matches the documented contract |
| Atomicity bug — claim_feedback row written but recommendation_claim feedback_state not updated | HIGH | Single transaction wrap via `with_claim_transaction`; integration test asserts both observe same state |
| Idempotent re-submission accidentally double-writes | MED | Explicit feedback_state check at step 2 of service flow; test asserts re-submission returns NoMutation |
| `BoundedNote` 200-char enforcement bypassed | LOW | Already enforced at `BoundedNote::try_from` construction; can't bypass without unsafe code |
| New `write.recommendations` scope isn't registered in WP allowlist → block can't call ability | MED | A8 channel enumeration covers this; L1 grep before merge |
| `Convert { Action }` writes both `ConfirmCurrent` AND an open-loop attachment — what if the open-loop service fails? | MED | Wrap both in the same transaction; if open-loop create fails, rollback the feedback too |
| `RecommendationFeedbackRecorded` signal fires before downstream consumers (W4-B/C) exist | LOW | Signal type declared but unused for one wave; no harm |

---

## §14 Definition of Done

Per CLAUDE.md DoD section:

1. All 5 `RecommendationFeedbackDecision` variants (10 mapping rows including Dismiss reasons) implemented per §1 table
2. End-to-end flow tested: ability invocation → `record_claim_feedback` → `claim_feedback` row written → `claim_feedback_recorded` signal fires automatically → `latest_feedback_suppression` returns the row on next read
3. No stubs or TODOs
4. `cargo clippy -- -D warnings && cargo test --lib && pnpm tsc --noEmit` clean
5. L2-status declared in commit messages
6. **No L4 evidence required for the W4-A merge itself** (no UI surface introduced by this lane). However: the `dailyos_suggested_next_steps_feedback_enabled` filter flip in the W3-A WP block is an **L4-gated change**, shipped separately from W4-A merge, gated by hands-on QA of the WP block → REST → ability → record_claim_feedback → claim_feedback row → surfacing read pickup vertical (B12).
7. **Privacy non-leak documented.** `EffectKind` carries no subject identity beyond what the caller already knows about its own claim (B12).
8. **Auth overhaul note.** Per the 2026-05-21 auth overhaul memory, ADR-0111 §193 nonce requirement for write events is being stripped for local same-user contexts. W4-A does NOT add nonce machinery; relies on the post-overhaul actor-allowlist model. L1 verifies the current state via a transport-layer grep (B12).
9. K-out: any class-pattern findings from L2 filed via `/ce-compound mode:headless` at retro close.
