---
title: "Fresh git worktree needs pnpm install before pre-push tsc gate fires"
problem_type: workflow_issue
track: knowledge
module: git worktrees, .githooks/pre-push (tsc gate)
component: development_workflow
severity: medium
tags: [worktree, pnpm, tsc, pre-push, setup-ritual]
date: 2026-05-19
related_linear: DOS-712
applies_when:
  - "Creating a new git worktree off origin/dev via git worktree add"
  - "About to push a Rust- or frontend-touching branch (pre-push gauntlet will fire)"
  - "Following the parallel-session fan-out setup from tasks/README.md or similar"
---

## Context

Surfaced during the 2026-05-19 v1.4.x W0 parallel-session fan-out (sessions 1–4, four worktrees forked off `origin/dev` in parallel). Session 4 hit this on the first push of PR #320.

The documented worktree setup ritual is:

```bash
git fetch origin dev
git checkout dev
git pull origin dev
git worktree add worktrees/<branch-name> -b <branch-name> origin/dev
ln -s ../../.claude worktrees/<branch-name>/.claude   # for hooks
cd worktrees/<branch-name>
```

`pnpm install` is **not** in the ritual. Worktrees inherit the repo's `.git/` and source files but **not** `node_modules/`.

## Guidance

**Add `pnpm install` to the worktree setup ritual.** Run it immediately after entering the worktree, before any tsc / lint / push command.

```bash
git worktree add worktrees/<branch-name> -b <branch-name> origin/dev
ln -s ../../.claude worktrees/<branch-name>/.claude
cd worktrees/<branch-name>
pnpm install                                          # <-- add this
```

For Rust-only changes you can skip it, but the pre-push hook runs `pnpm tsc --noEmit` on every push to this repo regardless of whether your diff touched frontend code, so you'll still need `node_modules/` to push successfully.

## Why this matters

The pre-push hook (`.githooks/pre-push`) runs the full gauntlet: `cargo clippy -- -D warnings`, `cargo test --lib`, and `pnpm tsc --noEmit`. Without `node_modules/`, the tsc step fails with the cryptic:

```
undefined
 ERR_PNPM_RECURSIVE_EXEC_FIRST_FAIL  Command "tsc" not found
```

The push is rejected. You've already spent 5–10 minutes letting clippy + tests run before tsc fires. That time is wasted on a setup gap, not a real code defect.

Worse: a manual `pnpm tsc --noEmit` in the worktree directory before pushing can mysteriously return `exit 0` with empty output (resolving against a parent-dir cache or stale node_modules elsewhere on the machine — mechanism unconfirmed), giving a false-clean signal. The pre-push hook then fails on the same command. Lesson: never trust a tsc exit code in a worktree without `node_modules/` actually present.

## When to apply

Every time you `git worktree add` for branch work that will be pushed. Skip only when:

- You're creating a worktree for read-only inspection (no push planned)
- The worktree's branch will never touch frontend or be Rust-pushed (rare — most branches do at least one)

## Examples

**Wrong (the gap):**

```bash
git worktree add worktrees/feature-x -b feature-x origin/dev
ln -s ../../.claude worktrees/feature-x/.claude
cd worktrees/feature-x
# edit, commit
git push -u public feature-x
# ❌ pre-push hook fails: Command "tsc" not found
```

**Right:**

```bash
git worktree add worktrees/feature-x -b feature-x origin/dev
ln -s ../../.claude worktrees/feature-x/.claude
cd worktrees/feature-x
pnpm install
# edit, commit
git push -u public feature-x
# ✅ pre-push hook runs the gauntlet against a real node_modules/
```

Sister gotcha: [`node-modules-tracked-symlink-enotdir-pnpm-install-2026-05-19.md`](node-modules-tracked-symlink-enotdir-pnpm-install-2026-05-19.md) covers the case where `node_modules` was tracked as a symlink in git — different root cause (CI side, not worktree side), same `pnpm install` failure mode.

## Tracking

- DOS-712 — Maintenance ticket for documenting `pnpm install` in the durable setup ritual (CLAUDE.md or equivalent) and optionally hardening the pre-push hook to fail fast on missing `node_modules/`.
