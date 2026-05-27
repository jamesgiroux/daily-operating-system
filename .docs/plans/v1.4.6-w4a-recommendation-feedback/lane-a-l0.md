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

## §1 The mapping (proposed; reviewer-challengeable)

Five `RecommendationFeedbackDecision` variants → ADR-0123 actions OR surfacing-state writes. **This is the load-bearing design decision of W4-A.**

| `RecommendationFeedbackDecision` | Maps to | Rationale |
|---|---|---|
| `Accept { at }` | `FeedbackAction::ConfirmCurrent` via `record_claim_feedback` | User confirms the recommendation is valid; reinforces source + agent reliability. Salience UserFit factor weights up on next compute. |
| `Dismiss { at, reason: NotRelevant }` | `FeedbackAction::WrongSubject` via `record_claim_feedback` | Per ADR-0123 §5: WrongSubject tombstones the claim on the asserted subject; source not punished. Matches "not relevant to this entity." |
| `Dismiss { at, reason: AlreadyKnew }` | Surfacing cooldown only (no `claim_feedback` write) | User isn't disputing the recommendation's truth — just doesn't need it surfaced. Bumps the cooldown on `(subject, action_signature)` per W2-A's surfacing policy. Does NOT downweight source or claim. |
| `Dismiss { at, reason: WrongSubject }` | `FeedbackAction::WrongSubject` via `record_claim_feedback` | Direct mapping; linker reliability hit, source untouched. |
| `Dismiss { at, reason: Other(BoundedNote) }` | Surfacing cooldown only + note logged to `claim_feedback` payload | Reason unknown; safest interpretation is "don't surface again on this subject" without truth-judgment. The note is captured for future analysis but doesn't drive any automatic semantic action. |
| `NotUseful { at }` | Surfacing cooldown only (no `claim_feedback` write) | Per DOS-332 AC: "Dismissed and not-useful recommendations reduce salience without necessarily reducing source trust." Treated as ranking-only signal — surfacing cooldown extends, UserFit downweights via W1-B salience re-compute. |
| `TooNoisy { at }` | Surfacing cooldown only (no `claim_feedback` write) | Per DOS-332 AC: "Too-noisy feedback updates surfacing thresholds/cooldowns rather than treating the claim as false." Extends cooldown more aggressively than NotUseful; possibly flags the entire subject's surfacing tier (subject becomes more conservative). |
| `Convert { at, into: Action(action_id) }` | `FeedbackAction::ConfirmCurrent` + open-loop attachment | Conversion is the strongest positive signal. The recommendation becomes an action (existing open-loop infrastructure); ConfirmCurrent reinforces the source. |
| `Convert { at, into: ClaimCorrection(claim_id) }` | `FeedbackAction::MarkOutdated` on the original + new claim proposal | The user is saying "the recommendation was based on outdated info; here's the corrected claim." MarkOutdated demotes the trigger claim; the corrected claim_id is the new ground truth. |
| `Convert { at, into: ReviewQueue(queue_item_id) }` | No `claim_feedback` write; existing review-queue routing applies | Per DOS-336 (review queue) the existing claim-review-queue lifecycle handles this. W4-A only emits the queue-item id — review queue owns the conversion semantics. |

**Cooldown-only writes go through W2-A's surfacing-policy service**, not through a new path. W4-A imports the cooldown API from `services::recommendations::surfacing` (which W2-A owns) and calls it with `(subject, action_signature, reason, decided_at)`.

**Updated `FeedbackState` after the call:** every variant transitions `Pending → Decided(<the variant>)` on the `RecommendationClaim`. The claim's `feedback_state` column updates atomically with the underlying action (whether that's a `claim_feedback` insert, a cooldown bump, or both).

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
- **Category:** `Write`
- **Allowed actors:** `[User, SurfaceClient]` — the user OR a surface acting on the user's explicit click. NOT `System` (no automatic feedback), NOT `Agent` / `Admin`, NOT `McpClient`.
- **`required_scopes`:** `["write.recommendations"]` (NEW scope; needs to be registered in the surface scope policy — see §5 channel enumeration)
- **`may_publish`:** false (writes don't publish to other surfaces; downstream effects are async via salience re-compute and surfacing-state mutation)
- **`mcp_exposure`:** `None`
- **`composes`:** `[claim_receipt]` (reads the receipt to verify the claim is in `Pending` feedback state before mutating; rejects double-feedback)
- **`signal_policy.emits_on_output_change`:** `[RecommendationFeedbackRecorded]` — a NEW SignalType variant that W4-A pre-declares for downstream W4-B/C consumption. Per wave plan §line 1018, the SignalType variant requires L0 approval — this packet is the approval request.
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

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubmitRecommendationFeedbackResponse {
    pub schema_version: u32,
    pub claim_id: ClaimId,
    pub feedback_state: FeedbackState,         // always Decided(<variant>) after success
    pub conversion_state: ConversionState,     // updated if Convert variant
    pub downstream_effect: DownstreamEffect,   // see below — explains what happened
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DownstreamEffect {
    ClaimFeedbackRecorded { action: FeedbackAction, feedback_id: String },
    SurfacingCooldownBumped { subject_kind: String, subject_id: String, action_signature: String, new_cooldown_until: DateTime<Utc> },
    BothApplied { action: FeedbackAction, feedback_id: String, new_cooldown_until: DateTime<Utc> },
    NoMutation { reason: String }, // e.g., already-decided idempotent reject
}
```

`DownstreamEffect` exposes which path the mapping took — useful for the surface to render a confirmation chip ("Action recorded" vs "Cooldown extended") and for tests to verify the mapping deterministically.

### §3.4 Service-side flow

`services::recommendations::feedback::record_recommendation_feedback`:

1. Load the `RecommendationClaim` by `claim_id`. Reject if not found (`ClaimError::UnknownClaimId`) or if not a recommendation claim (`ClaimError::UnsupportedClaimType`).
2. Check `feedback_state`. If already `Decided`, return `DownstreamEffect::NoMutation { reason: "already_decided" }` and the existing state. Idempotent semantics.
3. Match on `decision` variant per §1 mapping table; dispatch to either:
   - `record_claim_feedback(...)` with the mapped `FeedbackAction` and constructed `ClaimFeedbackInput`
   - `services::recommendations::surfacing::bump_cooldown(...)` with the subject + action_signature
   - Both (for the `Dismiss { Other }` and `Convert { ClaimCorrection }` variants)
4. Update the `RecommendationClaim` row's `feedback_state` and (if applicable) `conversion_state` columns atomically within the transaction.
5. Emit `RecommendationFeedbackRecorded` signal.
6. Return `SubmitRecommendationFeedbackResponse` with the relevant `DownstreamEffect`.

All within `with_claim_transaction` so the claim_feedback insert + recommendation_claim update + cooldown bump are atomic per claim.

---

## §4 Acceptance criteria

1. **Ability ships and is registered.** `submit_recommendation_feedback` in `tools/dailyos-abilities.json` (regen); `AbilityRegistry` exposes to `[User, SurfaceClient]`. `cargo test recommendations::submit_recommendation_feedback` passes.
2. **Mapping is deterministic.** Parametric test enumerates all 10 mapping rows in §1; asserts the documented `DownstreamEffect` shape for each.
3. **Idempotent.** Re-submitting feedback for an already-`Decided` claim returns `NoMutation` and does not mutate `claim_feedback` or surfacing state.
4. **Atomicity.** The recommendation_claim feedback_state column + the underlying action (claim_feedback row, cooldown bump, or both) are all in one transaction; partial mutations are not observable.
5. **No new tables.** Migrations diff is empty. Uses existing `claim_feedback` and surfacing state.
6. **No new ADR-0123 variants.** Maps to the existing 10 variants only. If reviewer panel finds any variant needs a new ADR-0123 mapping, L0 amendment first.
7. **Privacy.** `Dismiss { reason: Other(BoundedNote) }` enforces the 200-char cap at the API boundary (already enforced by `BoundedNote::try_from` in contracts.rs). The note is stored in `claim_feedback.payload_json`, never in surfacing rows.
8. **Signal contract.** `RecommendationFeedbackRecorded` SignalType variant declared in `signals/policy_registry.rs` (W0-style shared infrastructure file); downstream W4-B/C will reference it from this declaration.
9. **W3-A integration ready.** When `dailyos_suggested_next_steps_feedback_enabled` flips to true (post W4-A merge), the W3-A view.js handler POSTs to the WP REST bridge that invokes this ability with `Actor::SurfaceClient`. W4-A does not flip the filter; that's a one-line UI follow-up.
10. **Cycle hygiene.** `cargo clippy -- -D warnings && cargo test --lib && pnpm tsc --noEmit` clean.

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
| `tools/dailyos-abilities.json` regen | Deterministic per W3-A precedent |
| `signals/policy_registry.rs` `RecommendationFeedbackRecorded` variant | Shared infra file; commit alongside |
| `services::recommendations::feedback` doc comments | Document the mapping table inline so future readers don't have to find this packet |
| L2-status in commit messages | `passed` per memory rule |

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

## §12 Open questions (L0 panel decides)

1. **`Dismiss { Other(BoundedNote) }` mapping.** Proposed: cooldown bump + note logged to `claim_feedback.payload_json`. Alternative: treat as MarkOutdated with note. Which is the better default?
2. **`NotUseful` vs `TooNoisy` distinction.** Both are cooldown-only. Should TooNoisy bump the cooldown MORE aggressively (e.g., extend the whole subject's cooldown across action_signatures), or just bump the same as NotUseful? Per DOS-332 AC's "TooNoisy updates surfacing thresholds/cooldowns" — sounds like broader-than-NotUseful.
3. **Convert → ClaimCorrection mapping.** Currently MarkOutdated on the original + new claim proposal. Should the new corrected claim be auto-committed or stay in `propose` state per ADR-0123? Default: propose, per ADR-0123 §5.
4. **`RecommendationFeedbackRecorded` SignalType payload.** What fields does the signal carry? Minimal: `{ claim_id, decision_variant, downstream_effect_kind }`. Should also include `subject` for downstream filtering?
5. **WP REST endpoint shape.** The block needs a WP REST endpoint to POST feedback. Does it use existing dailyos-runtime-client transport with `Actor::SurfaceClient`, or a dedicated `/v1/surface/submit-feedback` endpoint? Default: reuse the runtime client invoke path (write abilities go through the same path; scope policy enforces).

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
2. End-to-end flow tested: ability invocation → service path → claim_feedback / surfacing state → signal emission
3. No stubs or TODOs
4. `cargo clippy -- -D warnings && cargo test --lib && pnpm tsc --noEmit` clean
5. L2-status declared in commit messages
6. No L4 evidence required (no UI surface in this lane)
7. K-out: any class-pattern findings from L2 filed via `/ce-compound mode:headless` at retro close
