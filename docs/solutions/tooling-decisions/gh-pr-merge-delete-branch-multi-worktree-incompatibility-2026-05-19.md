---
title: "gh pr merge --delete-branch fails when base branch is checked out in another worktree"
problem_type: tooling_decision
track: knowledge
module: gh CLI, git worktrees, PR merge workflow
component: development_workflow
severity: low
tags: [gh-cli, worktree, pr-merge, branch-delete]
date: 2026-05-19
applies_when:
  - "Merging a PR via gh CLI from a worktree (not the main repo dir)"
  - "The PR's base branch is already checked out in another worktree (typically the main repo dir on dev)"
  - "Using gh pr merge with --delete-branch flag"
---

## Context

Surfaced during 2026-05-19 parallel-session fan-out. Session 4 ran `gh pr merge 320 --merge --delete-branch` from `worktrees/v1.4.6-w0-adr-0125/` (with `dev` checked out in the main repo dir `/Users/jamesgiroux/Documents/dailyos-repo`).

The command failed with:

```
failed to run git: fatal: 'dev' is already used by worktree at '/Users/jamesgiroux/Documents/dailyos-repo'
```

The PR's API-side merge was likely complete; the local branch-delete step is what failed.

## Guidance

**When merging from a worktree, use bare `gh pr merge --merge` (no `--delete-branch`).** Delete the remote branch separately:

```bash
gh pr merge <pr-number> --merge

# After merge succeeds, delete the remote branch:
git push public --delete <branch-name>

# Local branch cleanup (from any worktree where the branch isn't checked out):
git worktree remove worktrees/<branch-name>
git branch -d <branch-name>   # -D if not yet locally merged
```

If you need `--delete-branch` semantics in one step, run `gh pr merge` from the main repo dir (or any clone where the base branch isn't `worktree add`-ed elsewhere).

## Why this matters

`gh pr merge --delete-branch` runs a local `git checkout <base>` step to clean up the merged feature branch. Git's worktree model forbids checking out a branch that's already checked out in another worktree (the same branch can't be in two places at once — would create conflicting indexes). When you run from a worktree, git's working dir is the worktree, and the base branch (typically `dev` or `main`) is checked out somewhere else.

The result is a confusing error from `gh`'s git invocation rather than a clean failure from `gh` itself. The PR's merge state on GitHub is fine; you just have to do the branch-cleanup steps manually.

## When to apply

- Every time you `gh pr merge` from a worktree (which is most of the time during parallel-session protocols where each session lives in its own worktree)
- When you see `fatal: '<branch>' is already used by worktree at '<path>'` from any `gh` command — same root cause applies broadly

## Examples

**Wrong (failed in session 4):**

```bash
cd worktrees/v1.4.6-w0-adr-0125
gh pr merge 320 --merge --delete-branch
# ❌ failed to run git: fatal: 'dev' is already used by worktree
```

**Right:**

```bash
cd worktrees/v1.4.6-w0-adr-0125
gh pr merge 320 --merge
# ✓ merged on GitHub

git push public --delete v1.4.6-w0-adr-0125
# ✓ remote branch deleted

cd /Users/jamesgiroux/Documents/dailyos-repo
git worktree remove worktrees/v1.4.6-w0-adr-0125
git branch -d v1.4.6-w0-adr-0125
# ✓ local cleanup
```

## Related

- The `git worktree remove` step itself can fail if the worktree has uncommitted changes. Use `--force` if you've intentionally discarded them.
- This is one of a family of "gh CLI + git worktrees" rough edges. The codex worktree isolation note ([codex-worktree-isolation-incompatible-with-rescue-forwarder-2026-05-18.md](codex-worktree-isolation-incompatible-with-rescue-forwarder-2026-05-18.md)) is unrelated mechanically but shares the underlying theme: tools that assume a single checkout misbehave under the parallel-worktree protocol.
