---
title: Silent-error-swallow class on signal emission — centralize via emit_or_log / emit_and_propagate_or_log wrappers
problem_type: architecture_pattern
track: knowledge
module: src-tauri/src/services/signals.rs (callers in services/, abilities/, clay/, context_provider/, executor.rs, google.rs)
tags: [silent-error-swallow, let-underscore, signal-emission, class-sweep, w4]
date: 2026-05-18
related_linear: DOS-5, DOS-216
---

## Context

W4 cycle 11 + cycle 12 L2 reviews surfaced the same pattern across the codebase: `let _ = ...emit_signal(...)` and `let _ = ...emit_signal_and_propagate(...)` call sites that silently discarded errors during signal emission. 40+ sites had drifted into this pattern. Each individual `let _ =` looks innocuous; collectively they create an observability blind spot where signal-emission failures leave no trace.

The class was previously partially-patched in earlier cycles by adding `if let Err(e) = ...` blocks at specific sites. That approach was correct shape but didn't scale — each new emit caller had to know to write the boilerplate, and the pattern would silently re-establish itself.

## Guidance

When a side-effect-returning helper is called from many sites and most callers want best-effort emission with observability rather than propagation, centralize the warn-log pattern as a wrapper exposed alongside the original API:

```rust
// services/signals.rs
pub fn emit_or_log(&self, signal: Signal) {
    if let Err(e) = self.emit_signal(signal) {
        warn!(error = %e, "signal emission failed");
    }
}

pub fn emit_and_propagate_or_log(&self, signal: Signal) {
    if let Err(e) = self.emit_signal_and_propagate(signal) {
        warn!(error = %e, "signal emission + propagation failed");
    }
}
```

Then sweep callers from `let _ = ...emit*()` to `*_or_log(...)`. Reserve direct `emit*` calls for sites that genuinely propagate errors via `?` or `.map_err(...)?`.

## Why This Matters

- **Observability:** silent-swallow loses both the error AND the fact that emission was attempted. The wrapper preserves the second signal via `warn!`.
- **Class enforcement:** a CI lint can detect `let _ = ... emit_signal` and direct callers can be reviewed against the "is propagation intentional?" question explicitly.
- **Drift resistance:** new callers reach for `emit_or_log` by name because the wrapper is in the same module as `emit_signal`. The wrapper makes the right thing easy.

## When to Apply

- Any place where 3+ callers have converged on `let _ = ...` against the same helper.
- Side-effect emitters where best-effort is the dominant caller intent (logging, telemetry, signal fanout, denormalized join writes).
- Migrations / repair jobs where a helper-emission failure should warn but not abort the surrounding operation.

Do NOT apply when callers genuinely should propagate (transactional invariants, audit-required writes, anything inside an outer `with_transaction` that must rollback on emission failure).

## Examples

W4 wave-pull commit `9fa8df99` swept 40+ sites. The class was caught structurally rather than per-site. Two `emit_propagate_and_evaluate` sites kept inline `if let Err` because they had a single caller — no wrapper indirection worth adding.

Sites with pre-existing `.map_err(...)?` were intentionally NOT swept — they were already error-propagating, and folding them into `*_or_log` would have flipped a propagating call to best-effort silently.

## Related

- Memory: `feedback_systemic_look_for_recurring_issue_classes` ("2+ similar findings = pause + class-level sweep")
- Memory: `feedback_zoom_out_for_class_pattern_in_l2_loop` ("same-shape finding in cycle-N+1 = stop patching, audit ENTIRE class, add structural gate")
- Memory: `feedback_two_similar_bugs_class_review`
