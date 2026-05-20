---
title: "Parallel-wave sub-PRs that all write to the same .synced-from file conflict on every merge"
problem_type: workflow_issue
track: knowledge
module: parallel wave coordination, .synced-from pin files, sync-from-canonical pattern
component: wave_orchestration
severity: medium
tags: [parallel-wave, synced-from, rebase-conflicts, sync-tooling, wave-coordination, chrome-lane]
date: 2026-05-19
last_updated: 2026-05-19
related_linear: DOS-729, DOS-730, DOS-731
applies_when:
  - "A wave is split into ≥2 parallel sub-PRs that each lift a verbatim chunk from a canonical source"
  - "Each sub-PR records its synced files in a shared `.synced-from` pin file (one per destination directory)"
  - "Sub-PR scopes are disjoint per-file but share the directory hosting the pin"
---

## Context

During the v1.4.4 W3 chrome-lane lift, three parallel sub-PRs each landed lifted assets in `wp/dailyos/theme/assets/chrome/`:

- **DOS-729 (W3-1)** — `design-tokens.css` + new `token-aliases.css` at `styles/`
- **DOS-730 (W3-2)** — 5 chrome `*.module.css` + fonts + `fonts.css` at `styles/` and `fonts/`
- **DOS-731 (W3-3)** — `chrome.js` + sync tooling at `assets/chrome/` (no `styles/` touch)

Each sub-PR wrote a `.synced-from` pin file recording the canonical SHAs of the files **it** had lifted. The convention was "one `.synced-from` per destination directory," which seemed clean per-file but turned out to be a shared write target across two of the three PRs (W3-1 and W3-2 both targeted `styles/.synced-from`; W3-2 and W3-3 both targeted `chrome/.synced-from`).

When W3-1 merged first, W3-2 hit `CONFLICT (add/add)` on `styles/.synced-from` — both branches added the file with disjoint content. Rebase + manual merge fixed it. Then W3-3 merged, and W3-2 immediately hit a second conflict on `chrome/.synced-from`. Another rebase + merge. Three rebases total to land three parallel PRs that should have had zero coupling.

## Guidance

When fanning out parallel sub-PRs that share a sync-from-canonical pattern, **don't share `.synced-from` files between sub-PRs**. Three patterns work; pick whichever fits the asset layout:

### Pattern A — one `.synced-from` per file (most defensive)

`design-tokens.css.synced-from`, `FolioBar.module.css.synced-from`, `chrome.js.synced-from`, etc. Each file pins itself, parallel PRs touch disjoint files, no conflicts.

Trade-off: more files in the destination tree; harder to scan at a glance. Worth it when the wave is ≥3 sub-PRs.

### Pattern B — pre-allocated `.synced-from` slots (lighter)

In the wave plan, name each sub-PR's `.synced-from` filename upfront and ensure disjoint slot allocation. E.g., `styles/.synced-from-tokens` (DOS-729 only), `styles/.synced-from-modules` (DOS-730 only). One slot per sub-PR.

Trade-off: filenames hint at wave structure (not always desirable); sub-PRs have to know their slot. Worth it when sub-PRs cleanly map to logical asset groups.

### Pattern C — write `.synced-from` only in the LAST PR of the lane (cheapest, requires sequencing)

W3-1, W3-2, W3-3 don't write `.synced-from` at all; W3-4 writes the consolidated pin file as part of its wire-up. Or a dedicated "consolidate pins" PR runs after.

Trade-off: gives up parallelism if the lane uses strict landing order, or trades it for a follow-up PR. Worth it when the lane already has a final wire-up step (chrome-lane W3-4 functions.php was such a step).

## Why this matters

Parallel waves are the whole point of the wave-orchestration primitive in `.docs/plans/{version}-waves.md`. Conflict-on-merge in the `.synced-from` file undermines that — each parallel PR adds a serialization point that should not exist. It also hides behind the false signal "your branch is up to date with public/dev" because the conflict is a content-level `add/add`, not detected until the merge attempt.

**Cost we paid this session:** ~10 min across 2 rebases + 2 force-pushes on DOS-730. Not catastrophic, but the next 5-way wave would have paid ~50 min in serialized rebase loops.

## Related

- `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` §10 W4-coupling matrix already asserts disjoint **file** write sets per parallel PR. Extending that assertion to disjoint **`.synced-from`** filenames would have caught this at L0.
- Memory entry `feedback_parallel_agent_sizing` — "mechanical + tight rule + non-overlapping files → fan out parallel codex agents". Add `.synced-from` filenames to the "non-overlapping" criterion.
- Migration slot reservation pattern (CLAUDE.md "Parallel-wave migration slot reservations") is the precedent — pre-reserved disjoint slots for parallel agents. Same shape applies to `.synced-from` pins.
