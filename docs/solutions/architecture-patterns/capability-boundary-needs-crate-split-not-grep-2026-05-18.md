---
title: Proc-macros and grep regexes cannot enforce capability boundaries inside a single crate — needs crate split
problem_type: architecture_pattern
track: knowledge
module: src-tauri/abilities (the proc-macro #[ability] + DOS-304 grep lint), src-tauri/src/lib.rs (workspace structure)
tags: [capability-boundary, proc-macro, lint, crate-boundary, dos-304, dos-349, w3]
date: 2026-05-18
related_linear: DOS-210, DOS-304, DOS-349
---

## Context

W3-A (DOS-210) shipped the `#[ability]` proc-macro and the DOS-304 grep-based lint script intended to enforce capability boundaries — abilities must declare which `services::*` mutators they can call. L2 cycle-1, L2 cycle-2, L3 cycle-1, and L3 cycle-2 all surfaced findings about porousness: module-scope `use` aliases, `use std::fs;` aliased imports, and re-export chains all bypass the grep regex.

The cycle-1 fix hardened the regex. The cycle-2 fix added AST visitor improvements. Cycle-2 review STILL returned BLOCK with three findings, all variations of the same root cause. The set-and-forget "cycle-2-still-BLOCKED" stop condition fired and escalated to L6.

**L6 ruling (Option A, 2026-05-01):** accept porous best-effort enforcement, file structural fix as DOS-349 (move ability runtime into a separate crate) targeted v1.5.x or a future wave. Hard precondition: must complete before any DOS-218+ migration (= before the first ability that actually uses the boundary).

## Guidance

**Single-crate capability boundaries are unenforceable.** Within one crate, Rust's module system gives every module visibility into every other module's full surface area. Proc-macros and grep regexes can detect direct uses but cannot eliminate the following bypass shapes:

- Module-scope aliased imports: `use crate::db as foo; ... foo::ActionDb::open()`
- Re-exports through a public module: `pub use crate::db::ActionDb;` in a sibling module
- Function-local aliases: `let f = crate::db::ActionDb::open;`
- Trait-imported method calls where the trait import grants method access

**The only structural enforcement is the crate boundary.** Move capability-restricted code into its own crate. Make `services::*` mutators a public API of the main crate; the abilities crate depends on it and can only call the public API. The compiler enforces visibility because the crate boundary is the visibility boundary.

## Why This Matters

- **Class-pattern recurrence:** L2 and L3 reviews kept catching variants of the same shape. This is a textbook example of memory `feedback_zoom_out_for_class_pattern_in_l2_loop` — same shape twice = class-wide structural fix, not a third patch.
- **False-confidence risk:** the grep + proc-macro combination produces a "lint passes" signal that doesn't actually constrain behavior. Worse than no lint, because reviewers trust the green check.
- **Honesty in the protocol:** the cycle-2-still-BLOCKED stop forced an honest L6 escalation rather than chasing increasingly-baroque regex extensions for another cycle.

## When to Apply

- Authoring or reviewing any new "capability allowlist" lint inside a single crate. Push back if the proposal is grep + proc-macro without a crate split path.
- Reviewing plans that name "DOS-304" or "capability boundary" — verify DOS-349 (crate split) is either already done or named as a precondition for the consumer that needs the boundary.
- L0 reviewing any plan that proposes to enforce a behavioral constraint via static analysis inside one crate. Ask: "what bypass shapes does this NOT catch?"

## Examples

W3-A cycle-2 escalation: "proc-macros and grep regexes can't enforce capability boundaries inside a single crate. Cycle-1 hardened the regex and the AST visitor, but the residual porousness (module-scope aliases, `use std::fs;` aliased imports) is not closeable without DOS-349's crate split."

DOS-349 was filed with priority HIGH, retargeted v1.5.x or future wave, with the precondition "must complete before DOS-218+ migration" (= the first consumer that actually needs the boundary).

## Related

- Memory: `feedback_zoom_out_for_class_pattern_in_l2_loop`
- Memory: `feedback_systemic_look_for_recurring_issue_classes`
- Memory: `feedback_set_and_forget_wave_protocol` (cycle-2-still-BLOCKED stop)
- DOS-304 (the porous lint, accepted as best-effort under L6 Path A)
- DOS-349 (the crate-split structural fix, blocking DOS-218+ migration)
