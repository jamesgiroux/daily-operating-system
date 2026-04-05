# I385 — Bidirectional Entity Hierarchy Signal Propagation — Upward Accumulation, Downward Fan-Out

**Status:** Open (0.13.3)
**Priority:** P1
**Version:** 0.13.3
**Area:** Backend / Signals

## Summary

The existing signal propagation model handles person → linked account propagation. This issue extends it to the entity hierarchy: signals from child accounts propagate upward to parent accounts (at 60% confidence), and significant parent-level signals fan out downward to all direct children (at 50% confidence, gated on ≥0.7 parent confidence). This enables: a pattern across multiple BUs to accumulate at the parent level and trigger portfolio re-enrichment; and a significant parent-level event (account-wide strategy shift, new executive sponsor) to cascade down to all BUs.

## Acceptance Criteria

From the v0.13.3 brief, verified with real data in the running app:

1. **Upward — basic propagation:** When a child account emits a signal, a derived signal is created for the parent account in `signal_events` within one propagation cycle. The derived signal has `source = 'propagation:hierarchy_up'` and `confidence = child_confidence * 0.6`. Verify with a known parent-child account pair.
2. **Upward — accumulation:** Two child accounts under the same parent each emit a signal of the same type within a 48-hour window. The parent's fused confidence for that signal type is higher than either individual derived signal. Verify by checking `signal_events` fusion result for the parent — the Bayesian log-odds fusion already handles this; confirm the outcome with real data.
3. **Upward — enrichment trigger:** When the parent's accumulated signals trigger the intel_queue (because new signals exist since last enrichment), the parent enrichment runs and produces updated portfolio intelligence. Verify: after two children emit signals, the parent's `entity_intel.updated_at` changes within one intel_queue cycle.
4. **Downward — fan-out:** When a parent account emits a signal with confidence ≥ 0.7, derived signals are created for all direct children in `signal_events` within one propagation cycle. Derived signals have `source = 'propagation:hierarchy_down'` and `confidence = parent_confidence * 0.5`. Verify with a known parent account by emitting a high-confidence signal.
5. **Downward — threshold gate:** A parent signal with confidence < 0.7 does NOT produce downward derived signals. Verify: emit a low-confidence signal on a parent; `signal_events` contains no `propagation:hierarchy_down` rows for its children.
6. **No loops:** Derived signals (source containing `propagation:hierarchy`) are not re-propagated. Verify: check `signal_events` after a propagation cycle — no `propagation:hierarchy` signal has itself spawned another `propagation:hierarchy` signal.
7. Downward propagation applies to direct children only. Grandchildren do not receive derived signals from this cycle. Verify: a three-level hierarchy (grandparent → parent → child); emit a signal on grandparent; only the parent (direct child) gets a derived signal, not the grandchild.

## Dependencies

- Independent — extends existing propagation rules, no other v0.13.3 issues block it.
- Unblocks I384 (portfolio intelligence) — portfolio intelligence is more meaningful with child signals already propagating to parent.
- See ADR-0087 decision 4.

## Notes / Rationale

The upward and downward propagation together solve the "signal isolation" problem in account hierarchies. Today, a risky signal at Cox B2B stays at Cox B2B — the parent entity (Cox Enterprises) never sees it unless the user explicitly tags the parent in a meeting. With upward propagation, that signal accumulates at the parent automatically. If three BUs each have moderate risk signals, the parent sees high accumulated confidence, triggering a portfolio-level intelligence refresh that surfaces the cross-BU pattern.
