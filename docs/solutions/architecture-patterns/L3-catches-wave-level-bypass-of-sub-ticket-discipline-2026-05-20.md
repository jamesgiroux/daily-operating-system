---
title: "L3 wave-level review catches cross-sub-ticket integration defects that L2 (per-PR AC-bounded) misses"
problem_type: architecture_pattern
track: knowledge
module: .docs/plans/engineering-ladder.md L2/L3 gates
tags: [l2, l3, wave-review, engineering-ladder, cross-sub-ticket, integration-defects, codex-challenge]
date: 2026-05-20
related_linear: v1.4.4 W1 L3 codex challenge cycle-1
related_memories: [l2-wave-scope-not-per-pr, l2-must-review-against-acceptance-criteria, zoom-out-for-class-pattern-in-l2-loop]
---

## Context

v1.4.4 W1 wave (10 substrate sub-tickets) passed L2 unanimously after 3 cycles. All 3 named L2 reviewers (`/review` + `code-reviewer` + `/cso`) approved cycle-2; codex cycle-3 P2s routed to path-α (DOS-749, DOS-750). Then L3 codex challenge surfaced 5 REAL defects that NONE of the L2 cycles had caught:

1. `MeetingPrepStatusChanged` signal coalesce policy not honored by emit path
2. Touchpoints bypass `build_receipt_for_audience` — raw meeting titles to AgentMcp (ADR-0108 violation)
3. v243 view recreation non-transactional (multi-process migration race)
4. Envelope cache lacks principal binding — cross-actor poisoning (ADR-0125 violation)
5. WP block consumer skeletons decorative — wrong `invoke_ability` signature; CI gate grepped names not signatures

Two of the five (#2 and #4) are real security/privacy contract violations. One (#5) is an AC-W1.2 violation that the CI gate signed off on falsely. L2 missed all of these because L2 is **per-sub-ticket AC-bounded** — each reviewer checked one sub-ticket against its own L0 packet ACs. The cross-cutting failure modes only show when the wave is viewed as a whole.

## Root cause

**L2 = per-PR / per-sub-ticket diff review.** Reviewers check: does this code fulfill the AC it was written for? Does it match its ADR contracts? Is there a regression in touched code?

**L3 = wave-as-a-whole.** Reviewers check: do the sub-tickets compose? Does signal A from ticket X interact correctly with signal B from ticket Y? Does the cache primitive from one ticket honor the discipline another ticket assumed?

This wave's L3 codex challenge prompt was deliberately scoped to 5 high-risk **integration points**, not to per-sub-ticket AC restatements. That focus is what surfaced the defects.

## Application

When dispatching L3 codex challenge:

- Prompt MUST target wave-level integration patterns, not per-sub-ticket implementation details
- Identify cross-sub-ticket coupling points BEFORE dispatch (signals across tickets, shared substrate, composable primitives, CI gate contracts)
- Use `codex exec` direct (per `feedback_codex_exec_direct_for_oneshot_reviews`) rather than `codex:adversarial-review` companion route — the latter OOM'd on the 19K-LOC wave diff; focused prompts via `codex exec` fit the context window
- Expect L3 to find defects L2 missed even when L2 was unanimous — the gates serve different purposes

When L3 finds wave-level defects:

- They are NOT path-α candidates (path-α is for theoretical hardening / not-AC-violations); L3 cross-cutting defects ARE substrate-fundamental
- Per `feedback_zoom_out_for_class_pattern_in_l2_loop`: if multiple cycle-1 L3 findings share a class-pattern (e.g., "wave-level bypass of per-sub-ticket discipline"), document the class and audit the entire substrate before patching, don't just patch the listed instances

## Class pattern surfaced

**"Wave-level bypass of per-sub-ticket discipline":** when each sub-ticket implements its own discipline (e.g., audience filtering, signal coalescing, principal binding) correctly within its own bounds, but the wave's integration glue (other sub-tickets that compose them) skips that discipline. Each sub-ticket is correct alone; the composition is wrong.

Audit pattern: for every primitive a sub-ticket declares as "must be applied at the boundary," grep the rest of the wave for callers that DON'T apply it. The bypasses are the wave-level defects.
