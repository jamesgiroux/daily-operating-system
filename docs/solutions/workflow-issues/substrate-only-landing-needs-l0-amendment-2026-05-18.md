---
title: "Substrate-only landing without L0 amendment is a protocol skip — costs 7x more to recover than honor up front"
problem_type: workflow_issue
track: knowledge
module: .docs/plans/v1.4.0-waves.md (Engineering Ladder L0–L6 protocol), .docs/plans/wave-WN/proof-bundle.md
tags: [protocol-skip, shape-only, scope-cut, l0-amendment, wave-protocol, w3]
date: 2026-05-18
related_linear: DOS-7, DOS-294, DOS-296, DOS-299, DOS-300, DOS-301
---

## Context

W3-C through W3-H (six substrate tickets) landed five commits on `dev` without per-PR L2 review. The catalyst was the appeal of "ship the shape, defer the wiring" — each substrate looked safe in isolation because no consumer was wired through it yet. Four substrates shipped substrate-only with significant deferrals.

The recovery cost: ~3.5 hours of retroactive L2 codex + architect-reviewer + code-reviewer + L3 codex challenge + L6 ruling + five remediation phases (DOS-300 fix, DOS-296 Uuid, DOS-294 schema reconciliation, DOS-299 backfill, DOS-301 projection).

The cost of honoring the protocol up front would have been ~30 minutes of L2 per commit × 5 commits = ~2.5 hours of parallel codex L2s + 30 min architect + 30 min code-reviewer.

**Recovery cost = ~7× protocol-honor cost.** Plus the meta cost of the L6 escalation and the wave-protocol skip becoming a retro topic.

## Guidance

- **"Shape-only is safe" is a trap.** A single shape-only substrate may be defensible in isolation. Four shape-only substrates simultaneously lock in shapes their future writers can't comfortably adopt. The integration becomes a liability before any consumer wires through.
- **Scope cuts require explicit L6 acknowledgment.** Each "deferred X to follow-up" in a commit message is a contract amendment. The L6 escalation policy's MUST-escalate trigger #6 (net-new scope expansion) and the "contract amendment" trigger cover this.
- **The proof-bundle template must call out scope cuts.** Adding a "Scope cuts taken (with L6 acknowledgment)" section to `proof-bundle.md` makes deferrals explicit. The current "Known gaps" section is too soft — it can be filled with "v1.4.1 candidates" without surfacing that they were unilateral cuts.
- **Per-PR L2 cost is the right ceiling.** ~30 min of parallel L2 per PR is the maximum tolerable cost. Anything above that is wave-recovery territory.

## Why This Matters

The protocol exists because retroactive review costs more than synchronous review. The "ship now, review later" pattern is appealing under deadline pressure but the math doesn't work out. The L6 ruling for W3-C/H validated this (Path A: land fixes on dev, not file as v1.4.1) — but the cleaner path was to never skip in the first place.

## When to Apply

- Any wave where 2+ tickets are "substrate-only with deferred consumer."
- Any commit with "deferred X" in the message — must be L6-acknowledged before landing.
- Any wave-recovery situation where retroactive L2 is being considered as a default rather than an exception.

## Examples

**W3-C through W3-H recovery (2026-05-02 → 2026-05-03):**
- Off-protocol initial landing: 5 commits on dev
- Retroactive L2 codex × 5 commits: ~30 min wall-clock parallel
- Retroactive architect-reviewer + code-reviewer: ~30 min each
- L3 wave adversarial codex: ~10 min
- L6 ruling: ~5 min user decision (Path A)
- Five remediation phases (DOS-300, 296, 294, 299, 301): ~3.5h codex + ~1h orchestrator

**Compared to honoring protocol per commit:** ~30 min × 5 + integration cost = ~2.5h total, no L6 escalation needed, no proof-bundle "wave-recovery" section.

## Related

- Memory: `feedback_l2_is_not_optional`
- Memory: `feedback_l2_wave_scope_not_per_pr`
- Memory: `feedback_no_deferrals_period`
- Memory: `feedback_wire_existing_substrate_not_future_producer`
- `.docs/plans/engineering-ladder.md` § L2 Diff pass rule
