# DOS-428 - Claim-Lifecycle SignalDot Wire-In - L0 Plan
**Wave:** W4 redesign
**Status:** L0 draft
**Dependency:** DOS-414 must complete first. This ticket is inert until Moving signals exist.

## 1. Ticket Reference And Acceptance Summary
[DOS-428](https://linear.app/a8c/issue/DOS-428) wires claim lifecycle state into Moving signal rendering.

Acceptance criteria:
- [ ] Every claim-backed `MovingSignalViewModel` emitted by the Moving composer carries `correctionState`.
- [ ] A signal whose backing claim was user-corrected emits `correctionState: "corrected"`.
- [ ] A signal whose backing claim is contested emits `correctionState: "contested"`.
- [ ] A claim-backed signal with no correction or contest emits `correctionState: "none"` or intentionally omits the field only if the existing serde contract allows omission.
- [ ] Non-claim-backed signals do not invent lifecycle state.
- [ ] `SignalDot` receives the state through the existing `MovingSignalViewModel` prop and applies the existing corrected/contested modifier classes.
- [ ] Tests cover corrected, contested, and none/default lifecycle mappings on serialized Moving signals.
- [ ] No SignalDot CSS, token, or inline style work lands in this ticket.
- [ ] No new ephemeral ticket IDs land in code comments.

## 2. What This Builds
This ticket builds the read-side wire between the claim substrate and the Moving composer.

The implementation adds a lifecycle lookup in `src-tauri/src/services/briefing/moving.rs` so each signal sourced from an intelligence claim can set the flattened Rust `LifecycleMixin` on `MovingSignalViewModel`.

Already present:
- `src/types/briefing.ts` defines `LifecycleMixin` and `MovingSignalViewModel extends TrustMixin, LifecycleMixin`.
- `src-tauri/src/services/briefing_view_model.rs` defines `LifecycleMixin`, `CorrectionState`, and flattened Moving signal serde.
- `src/components/dashboard/SignalDot.tsx` maps `signal.correctionState === "corrected"` to `styles.corrected`.
- `src/components/dashboard/SignalDot.tsx` maps `signal.correctionState === "contested"` to `styles.contested`.
- `src/components/dashboard/SignalDot.module.css` owns the W1 visual treatment.

No frontend type expansion, display parsing, or CSS branching should be needed.

## 3. Trust-Source Declaration
Trust source for lifecycle state: the DOS-411 claim lifecycle substrate.

In this branch, the concrete source is the `intelligence_claims` table plus `claim_feedback` audit history, not a separate `claim_lifecycle` table.

Named source fields:
- `intelligence_claims.verification_state` is the source for contested state.
- `intelligence_claims.claim_state`, `surfacing_state`, `demotion_reason`, `retraction_reason`, and `superseded_by` are the source for corrected/superseded lifecycle state.
- `claim_feedback.feedback_type` is audit history only for this composer. The composer should not infer current UI state by grepping historical feedback rows.

If the parent DOS-411 track exposes the same state through a `claim_lifecycle` table or view before implementation, that table/view becomes the named source. The rule stays the same: read normalized current lifecycle state, not raw event history.

Mapping:
- `verification_state = 'contested'` or `verification_state = 'needs_user_decision'` maps to `CorrectionState::Contested`.
- `demotion_reason = 'superseded'` or `superseded_by IS NOT NULL` maps to `CorrectionState::Corrected`.
- Everything else maps to `CorrectionState::None`.

Precedence:
- Contested wins over corrected if both are present.
- Corrected wins over none.
- None is the default for active, uncontested claim-backed signals.

## 4. Coordination With DOS-414
DOS-414 owns Moving aggregation and signal emission. DOS-428 only matters after DOS-414 creates real `MovingSignalViewModel` rows from upstream events.

Required DOS-414 interface:
- Each claim-backed signal has a stable `claim_id` available while composing the view model.
- Non-claim signals are distinguishable without parsing rendered copy.
- The composer can batch lifecycle lookups by claim ID.

Sequencing:
1. DOS-414 lands the Moving aggregation pipeline and preserves backing claim IDs.
2. DOS-428 consumes those claim IDs and populates `LifecycleMixin.correction_state`.
3. DOS-428 does not create producers, ranking, grouping, entity selection, or thread actions.

If DOS-414 does not expose claim IDs, DOS-428 blocks and returns to L0/L1 for interface revision. Do not recover by matching rendered text to claims.

## 5. Files
Implementation file:
- `src-tauri/src/services/briefing/moving.rs` adds a batched claim lifecycle lookup helper, maps normalized claim state to `CorrectionState`, threads `LifecycleMixin` into claim-backed `MovingSignalViewModel` rows, and rewrites any touched module comments to durable dependency names instead of ephemeral ticket IDs.

Likely test file:
- `src-tauri/src/services/briefing/moving.rs` adds mapping unit tests and serialization tests for corrected and contested `correctionState`.

Reference-only files:
- `src/types/briefing.ts` - no change expected.
- `src-tauri/src/services/briefing_view_model.rs` - no change expected unless DOS-414 changed signal construction.
- `src/components/dashboard/SignalDot.tsx` - rendering already exists.
- `src/components/dashboard/SignalDot.module.css` - CSS already exists from W1.

## 6. Implementation Shape
Add an internal `ClaimLifecycleRow` in `moving.rs` with: `claim_id`, `verification_state`, `claim_state`, `surfacing_state`, `demotion_reason`, `retraction_reason`, and `superseded_by`.

Add helpers:
- `load_claim_lifecycle_states(db, claim_ids) -> HashMap<String, CorrectionState>`
- `correction_state_for_claim(row: &ClaimLifecycleRow) -> CorrectionState`
- `lifecycle_for_signal(claim_id, lifecycle_map) -> LifecycleMixin`

Lookup behavior:
- Deduplicate claim IDs before reading.
- Return only requested claim IDs.
- Treat missing claims as `CorrectionState::None` with a debug/error log, not a panic.
- Avoid one database round trip per signal.

Signal construction behavior:
- Use the DOS-414 claim ID before dropping source-specific signal structs.
- Set `LifecycleMixin { correction_state: Some(mapped_state) }` on claim-backed signals.
- Keep non-claim signals absent or `None` consistently with existing serde expectations.

## 7. Display-Layer Purity
`SignalDot` remains a pure renderer:
- It must not query claims.
- It must not inspect feedback history.
- It must not derive lifecycle state from text.
- It must not add inline CSS.
- It must not add new modifier class names.

The composer owns data derivation; the primitive owns visual mapping.

## 8. Out Of Scope
- DOS-411 user-note claim type, migration, feedback semantics, and lifecycle cutover. That already shipped on the parent track.
- DOS-414 Moving aggregation, ranking, entity grouping, source adapters, and signal emission.
- SignalDot CSS or token changes. W1 already shipped corrected/contested classes.
- New correction or contest mutations.
- Watch section lifecycle propagation.
- Any frontend behavior beyond consuming the existing `correctionState` field.

## 9. Risk And Rollback
Risks:
- DOS-414 may emit signals without stable claim IDs. Mitigation: block until the interface is explicit.
- Lifecycle mapping may drift from DOS-411 semantics. Mitigation: centralize mapping in one helper and unit-test normalized states.
- Query shape may become N+1 under real signal volume. Mitigation: batch by unique claim IDs.
- Corrected plus contested can coexist. Mitigation: contested precedence is explicit.

Rollback:
- Remove the lifecycle lookup and helper call from `moving.rs`.
- Leave `SignalDot`, types, and CSS untouched.
- Moving signals render without lifecycle modifier classes.

## 10. L1 Self-Validation Gates
- `cargo test briefing::moving` or the narrow Moving composer equivalent passes.
- `cargo test briefing_view_model::serde_roundtrip` passes if the fixture is touched.
- A corrected claim fixture serializes a Moving signal with `"correctionState":"corrected"`.
- A contested claim fixture serializes a Moving signal with `"correctionState":"contested"`.
- A normal claim fixture serializes as `"correctionState":"none"` or intentionally omits the field per existing serde.
- Non-claim signals do not show corrected or contested state.
- `pnpm tsc --noEmit` remains clean because frontend contracts should not change.
- `rg -n "DOS-[0-9]+" src-tauri/src/services/briefing/moving.rs src/components/dashboard/SignalDot.tsx` shows no newly introduced code-comment references.
- `rg -n "style=|<style" src/components/dashboard/SignalDot.tsx src-tauri/src/services/briefing/moving.rs` shows no inline CSS additions.

## 11. L2 Review Gates
Reviewers:
- Rust/service reviewer: batching, lifecycle mapping, missing-claim handling, and serialization.
- Product/data reviewer: corrected/contested semantics match the DOS-411 current lifecycle source.
- Frontend/design reviewer: no SignalDot CSS or rendering contract changes.

Required L2 questions:
- Does the implementation read normalized current lifecycle state instead of historical feedback rows?
- Does contested precedence hold when a superseded claim is also contested?
- Does DOS-414 provide claim IDs early enough in the composer path?
- Does the implementation avoid text matching, display parsing, and source-specific special cases?
- Are ticket IDs absent from touched code comments?

Pass rule: all L2 reviewers approve, or the plan returns to L1 with a concrete correction list.
