# ADR-0138: Unified claim feedback — `record_claim_feedback` as the single feedback writer

**Status:** Proposed
**Date:** 2026-06-10
**Amends:** [ADR-0123](0123-typed-claim-feedback-semantics.md) effect-tuple matrix (absorbs legacy entity-feedback side effects)
**Supersedes (on completion):** the `submit_intelligence_correction` write path in `services/feedback.rs` as a feedback writer; `entity_feedback_events` becomes a read-only correction-history ledger

## Context

DailyOS has two feedback substrates, built in different eras:

1. **Entity-feedback path** (pre-claims). `submit_intelligence_correction` (`services/feedback.rs`) targets *fields on entities* (`"health"`, `"state_of_play"`, now `"composition:sections.N.blocks.N..."`). Five actions (`confirmed` / `rejected` / `annotated` / `corrected` / `dismissed`). Side effects: `entity_feedback_events` row, Bayesian `signal_weights` update, `intelligence_*` signal emission, annotation→intel-prompt threading, suppression tombstones (+ shadow tombstone claims), health recalc on health-affecting account fields, self-healing enrichment correction.

2. **Typed claim-feedback path** (ADR-0123). `record_claim_feedback` (`services/claims.rs` + `claim_feedback_propagation.rs`) targets *claims*. Ten typed actions, each mapping to a deterministic effect tuple: claim lifecycle, verification-state machine (active → confirmed/contested), corroboration rows, claim-type source-reliability deltas, repair queue, render policy.

The two systems do not duplicate each other's trust math — they feed **different halves of the Intelligence Loop with different writers**:

- Entity path → `signal_weights` → consumed by the **generation side** (`intelligence/prompts.rs` `signal_weights_block` threaded into intel passes; self-healing).
- Typed path → `claim_type_source_reliability` → consumed by the **scoring side** (`trust_recompute.rs` → Trust Compiler → trust scores/bands).

The consequence: the same human judgment does half a job depending on which UI door it enters. Confirming via a composition block (`BlockFeedback` → `IntelligenceCorrection`) rewards the source in prompt context but never reaches the Trust Compiler; the claim's `verification_state` never becomes `confirmed`, so a confirmed-by-user inferred claim still renders faded (trust-as-opacity keys off `provenance_kind`). Conversely, typed-path feedback never updates `signal_weights`, so the generation side doesn't learn from it. The composition UI already extracts `claim_refs[0].claim_id` — and then writes it into the wrong system's `item_key`.

`claim_feedback_propagation.rs` already asserts the end state in its module docs: "`record_claim_feedback` remains the single feedback writer." This ADR makes that sentence true.

## Decision

**One writer, one event, fan-out via effect tuples.** `record_claim_feedback` becomes the only feedback write path. The legacy side effects move into the ADR-0123 effect-tuple matrix — the extension point that matrix was designed for — so one judgment feeds both the generation side and the scoring side, transactionally.

### 1. Effect-tuple absorption

- `ConfirmCurrent` additionally upserts `signal_weights` (alpha++) for the attributed source.
- Contest-class actions (`MarkFalse`, `NeedsNuance`, `WrongSource`) additionally upsert `signal_weights` (beta++), run the self-healing enrichment-correction hook, and trigger health recalc / snapshot propagation where the claim backs a health-affecting account field.
- `intelligence_*` bus signals continue to emit from the unified writer (signal vocabulary preserved; emitter moves).

### 2. Legacy action mapping

| Entity action | Typed action | Notes |
|---|---|---|
| `confirmed` | `ConfirmCurrent` | corroboration row + `verification_state → confirmed`; opacity lift falls out for free |
| `corrected` | `NeedsNuance` | original dormant + superseded by a **user-authored claim** with user provenance |
| `dismissed` | `MarkFalse` **or** `NotRelevantHere`/`SurfaceInappropriate` | the old action conflated "wrong" with "don't show here"; the typed path forces the split — wrongness penalizes the source, placement complaints do not |
| `annotated` | user-authored **context claim** on the subject | annotations stop being a side-channel note; `build_intelligence_context()` picks them up natively, replacing bespoke prompt-threading |
| `rejected` | retired | no UI emits it |

### 3. Targeting

Composition surfaces pass `claim_refs` — the bridge is mechanical. Legacy field-targeted surfaces (triage cards, divergences, work suggestions) go through a field→claim resolution adapter (extending `resolve_intelligence_source`) until they migrate to compositions.

### 4. Snapshot propagation (the R4 problem)

The WR R4 inline-edit route chose `update_account_field` because the claim path "wouldn't propagate the snapshot column." The durable fix lives here: a propagation target in the claim-feedback worker materializes vitals-backed claim changes into the authoritative account columns. Account columns trend toward projections of claims.

### 5. Ledger disposition

`entity_feedback_events` is dual-written during migration for correction-history continuity, then frozen as a read-only ledger. No history migration is required.

## Sequencing

- **Phase 1** — bridge composition `BlockFeedback` to `submit_claim_feedback` via the action mapping; dual-write `entity_feedback_events`. Closes the WR confirm→opacity gap.
- **Phase 2** — absorb legacy side effects into effect tuples (`signal_weights`, self-healing, health recalc via claim→snapshot propagation); annotations become context claims.
- **Phase 3** — migrate remaining legacy surfaces as they move to compositions; freeze the old writer.

## Consequences

- A confirm or contest anywhere in the product updates claim verification state, trust scoring, source reliability, prompt context, and rendering coherently — the feedback loop the Intelligence Loop check (question 5) requires, with no path dependence on which surface the user happened to be on.
- DOS-853 (trust cold-start calibration) benefits directly: confirmations finally reach the Trust Compiler, so the score trains as designed.
- The typed action vocabulary forces an intent split (`wrong` vs `hide`) the old UI papered over; the UI prompt copy stays, the wire format sharpens.
- Until Phase 3 completes, legacy surfaces keep their current behavior through the adapter — no flag-day cutover.
