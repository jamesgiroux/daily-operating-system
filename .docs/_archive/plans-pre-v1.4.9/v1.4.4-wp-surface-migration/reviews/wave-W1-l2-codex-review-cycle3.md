# L2 Codex Review — v1.4.4 W1 Wave (cycle 3)

**Reviewer:** `/codex review` (codex-companion.mjs review)
**Diff:** `0f8533e1..ea05ddc6` (full wave including cycle-1 + cycle-2 patches + cycle-3 P2 patches)
**Date:** 2026-05-20

## Verdict: 2 P2 findings (no verdict line); routed to path-α per James 2026-05-20

## Findings (both P2, both in DOS-8 feedback path)

### [P2] Idempotency race — `services/claim_receipt/feedback.rs:323-333`

The cache is checked before the DB write and only populated after the write completes, so two identical submissions arriving concurrently within the 60s window can both miss and both call `record_claim_feedback`, producing duplicate feedback rows/jobs despite the idempotency contract. Use an atomic get-or-reserve/in-flight entry or a DB uniqueness guard for the scope.

### [P2] lifecycle_changed computed from verification state — `services/claim_receipt/feedback.rs:362`

`lifecycle_changed` compares the new verification state to the old verification state, so lifecycle-only actions like `mark_outdated`, `mark_false`, `wrong_subject`, or `needs_nuance` return `lifecycleChanged: false` even though claim_state/surfacing_state changed and the receipt should update. Track the lifecycle outcome from the writer or derive it from the action/result instead of reusing verification state.

## Routing

Per memory `feedback_l2_path_alpha_to_maintenance_project` + James's decision 2026-05-20: neither finding is a literal AC-8 violation. The 3 named L2 reviewers (`/review` + `code-reviewer` + `/cso`) all approved cycle-2; the engineering ladder pass rule is met. Both findings filed as Codebase Maintenance project tickets:

- Maintenance ticket #1: Idempotency race (atomic reserve OR DB uniqueness guard)
- Maintenance ticket #2: lifecycle_changed accuracy for lifecycle-only action variants

Per memory `feedback_zoom_out_for_class_pattern_in_l2_loop`: both findings are in the DOS-8 feedback path — a class-wide sweep on feedback-path edge cases is the right structural response, filed as a separate maintenance follow-up.

L2 cycle 3 closes; proceed to L3.
