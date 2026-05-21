---
title: "Premise-check whether a 'production bug' is dev-environment friction in disguise before scoping wave-level work — multi-worktree port churn, continuous rebuilds, and hot-reload often inflate symptoms beyond what real users hit"
problem_type: workflow_issue
track: knowledge
module: .docs/plans/v1.4.3-waves.md (§W5 collapsed), CLAUDE.md (wave-protocol scoping)
tags: [scoping, ticket-scope-reset, dev-vs-production, premise-check, w5, dos-727, dos-733]
date: 2026-05-20
related_linear: DOS-727, DOS-733
related_memories: [feedback_premise_check_production_vs_dev_friction, feedback_review_loop_diminishing_returns_means_scope_is_wrong]
---

## Context

v1.4.3 W5 was originally scoped as **Studio Sandbox Compatibility (C3)** — port stability + mDNS service discovery + sentinel-port drift mitigation. The wave plan named the symptom: "Studio sandbox restart after Tauri restart causes WP block render to hit a stale sentinel port."

Reproduced live during the session:
- Studio sandbox paired to runtime URL `http://127.0.0.1:50633`
- That port was dead (Tauri not running)
- Sentinel file absent at `~/.dailyos/runtime-endpoint.json`
- `GET /accounts/acme-corporation/` rendered HTTP 200 with 6 `is-empty` block instances

The "stale-sentinel" framing was real but the production impact was overstated. The user pushed back: **"are these issues actual issues for production or the reality of working in multiple branches and continuously updating and hot reloading the tauri app as we push new code? is this an edge case worth considering?"**

## The dev-vs-production analysis

| Failure mode | Production frequency × visibility | Dev-environment frequency × visibility |
|---|---|---|
| Cold start (Studio open before Tauri) | High — every onboarding + every machine reboot | Constant during work |
| Tauri crash / auto-update restart | Medium — recovery moment with no feedback | Frequent via `pnpm dev` rebuild |
| Studio sandbox restart while Tauri alive | Low | Medium |
| Sleep/wake | Low (transient) | Low |
| Multi-worktree port churn | None (production users have one Tauri) | High |

The wave plan's mDNS / port-pinning emphasis was solving the bottom row of that table. Real production users restart Tauri rarely (auto-update, machine reboot, occasional crash) and don't have continuous CI rebuild churn.

The genuine production failure mode was **cold-start UX**: silently empty blocks when the runtime is down. That fix turned out to be a single missing filter registration in the WP plugin (`dailyos_runtime_client_for_block` filter never installed by the block-registration render path).

## Resolution

- **W5 wave collapsed** into single-PR fix DOS-733
- **PR #333** registered the filter globally at plugin init priority 5; class-wide CI gate added to prevent regression
- Production cold-start now shows actionable `runtime_unavailable_notice` instead of silent `is-empty`
- `.docs/plans/v1.4.3-waves.md` §"Wave 5 — COLLAPSED" documents the rationale

## Signals to detect before scoping a wave

1. **Symptom reproduces consistently in your dev workflow.**
2. **Reproduction requires multi-worktree / rebuild-loop / multi-day-session conditions** the production user wouldn't naturally hit.
3. **The wave-plan deliverables are mostly dev-ergonomics improvements** (port pinning, discovery protocols, restart resilience) rather than user-visible UX fixes.

## How to apply

Before writing acceptance criteria for a wave-level scope:

1. **Build a production-vs-dev impact table** like the one above. Columns: failure mode, production frequency × visibility, dev-environment frequency × visibility.
2. **Score each row.** If the dev column dominates and the production column is thin, the scope should shrink.
3. **Identify the genuine production failure mode** — it's often a single user-visible UX gap (silent failure, missing diagnostic, unactionable error).
4. **File that as a focused ticket.** Wave-level scope is for substrate work that genuinely needs multi-ticket coordination, not for dev-ergonomics dressed up as user-facing.

## Don't do this

- Don't take "I reproduced it" as proof of production impact. Your dev workflow has 5 worktrees + hot reload + continuous rebuilds; production has one Tauri install + auto-update.
- Don't conflate "the substrate could be more robust here" with "users are hitting this." Robustness improvements file as path-α or maintenance tickets; user-visible UX fixes file as focused tickets.

## Do this

- **Premise-check before AC.** The 10 minutes spent on the impact table saves the L0 review-loop cycles that would have followed.
- **Pair with [[l0-review-loop-diminishing-returns-means-scope-is-wrong]]**: ungrounded scope is what produces the review-loop diminishing returns.
- **Pair with [[feedback_check_substrate_before_authoring_primitives]]**: substrate often already does most of what the wave thinks needs net-new work.

## Cross-references

- Memory: `feedback_premise_check_production_vs_dev_friction`
- Wave plan: `.docs/plans/v1.4.3-waves.md` §W5 (COLLAPSED)
- Retro: `.docs/plans/v1.4.3-wp-foundation/retro.md` §"W5 collapsed → DOS-733"
