---
title: "Tracked node_modules symlink causes ENOTDIR on every CI pnpm install — git rm --cached removes the silent dev-CI killer"
problem_type: workflow_issue
track: knowledge
module: .gitignore, .githooks/pre-commit (untracked-add safety), .github/workflows/lint-frontend.yml (pnpm install step)
tags: [pnpm, ci, enotdir, symlink, git-tracking, lint-frontend, w2, w3]
date: 2026-05-19
related_linear: DOS-698
---

## Context

v1.4.3 W3 PR-E1 CI `frontend` + `lint-and-checks` jobs both red with:

```
ENOTDIR  ENOTDIR: not a directory, mkdir '/home/runner/work/daily-operating-system/daily-operating-system/node_modules'
pnpm: ENOTDIR: not a directory, mkdir '/home/runner/work/.../node_modules'
##[error]Process completed with exit code 236.
```

Looked like a flaky CI infra problem — `pnpm install` retried, kept failing identically. Reproducible, not flaky.

## Root cause

`git ls-files | grep node_modules` returned a tracked entry:

```
$ git ls-files -s node_modules
120000 28bd9f6fae993c5b3c26576c456dfebd7ae36cc7 0  node_modules
```

Mode `120000` = symlink. Content: `/Users/jamesgiroux/Documents/dailyos-repo/node_modules` — an absolute path on the original author's macOS workstation. Accidentally committed in PR #308 (W2 PR-D2 Pill block, 2026-05-18) — probably staged via `git add -A` or `git add .` with a stale symlink in the worktree.

On CI runners, the absolute path doesn't exist, so the symlink resolves to nothing. `pnpm install` calls `mkdir node_modules` — the path is already occupied by a dangling symlink, so `mkdir` returns `ENOTDIR` (not a directory).

`node_modules` is in `.gitignore` (`62:node_modules/`) — but `.gitignore` does NOT untrack already-tracked entries. The symlink was tracked BEFORE the gitignore line and stayed.

## Fix

```
git rm --cached node_modules
```

Removes from the index; leaves the local file alone (the symlink is gitignored going forward). Committed + pushed; CI `frontend` job flipped to PASS on next run.

`dev` was red on this for **3+ consecutive runs** before PR #315 surfaced it. CLAUDE.md path-trigger filters on lint-frontend.yml mean lots of PRs land without touching the frontend lint surface, so the breakage stayed invisible.

## Why pre-commit hooks didn't catch it

The pre-commit gate (`.githooks/pre-commit`) gates on:

- PII blocklist + stub/TODO scan + schema sync + ADR-0101 boundary + reference fidelity
- Clippy + cargo test --lib (only if `.rs`/`Cargo.*` staged)
- `pnpm tsc --noEmit` (only if `.ts`/`.tsx`/`.js`/`.jsx`/`package.json` staged)

A symlink at the repo root is **none of those file types**. No gate fires.

## Pattern: prefer specific `git add` over `git add -A` / `git add .` near symlinks

The accidental commit pattern is `git add -A` (or `git add .`) when the local workdir has a stale or developer-local symlink at the repo root. Two mitigations:

1. **Pre-commit symlink scan.** Add to `.githooks/pre-commit`:
   ```bash
   # Block staging tracked symlinks at the repo root that look developer-local.
   git diff --cached --name-only --diff-filter=A \
     | xargs -I {} test -L {} && {
       echo "STAGED SYMLINKS DETECTED:"
       git diff --cached --name-only --diff-filter=A | xargs -I {} ls -la {} | grep '^l'
       exit 1
     }
   ```
   (Or narrower: only block at repo root; symlinks deeper in the tree may be legitimate.)
2. **Routine `git ls-files -s | grep '^120000'` audit** at L3 retro to spot tracked symlinks before they ship.

## Cost

- ~30 min spent diagnosing "flaky CI ENOTDIR" before finding the tracked-symlink root cause.
- Multiple developers (or PRs) since W2 PR-D2 silently hit this on `dev` lint-frontend.yml without it blocking merges (path-filter shielded most PRs).

## Cross-references

- PR #308 (W2 PR-D2 Pill block) — accidental commit
- PR #315 (W3 PR-E1) — discovered + fixed via commit 11 (`v1.4.3 W3 commit 11: remove accidentally-tracked node_modules symlink`)
- Memory: `feedback_post_rebase_integration_damage_blind_gates.md` — same class (silent dev breakage invisible until first PR touches gated path)
- DOS-696 (post-rebase integration damage class) — the broader pattern this belongs to
