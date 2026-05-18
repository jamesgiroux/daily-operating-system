---
title: Codex Agent isolation:worktree mode auto-prunes worktrees that don't accumulate file changes — incompatible with rescue-forwarder pattern
problem_type: tooling_decision
track: knowledge
module: Agent tool isolation:worktree, codex-rescue subagent, node companion.mjs
tags: [codex, worktree, agent-isolation, rescue-forwarder, parallel-dispatch, w3]
date: 2026-05-18
related_linear: (W3-C/H recovery)
---

## Context

W3-C/H attempted parallel codex dispatch via the `Agent` tool with `isolation: "worktree"`, expecting each worktree to host its own codex job making file changes. Three of four parallel jobs died silently mid-run.

**Root cause:** the Agent tool's `isolation:worktree` mode auto-prunes worktrees that don't accumulate file changes (the agent that owns the worktree is what makes changes, and worktrees with no diff are pruned for hygiene). The rescue-forwarder pattern uses a thin Agent wrapper to dispatch codex jobs — but the *forwarder agent itself* doesn't write files; the codex job running inside the worktree does. From the Agent tool's perspective, the forwarder agent made no changes, so the worktree was eligible for auto-prune. The worktree got cleaned up mid-job while the underlying codex process still pointed at the deleted directory.

## Guidance

**Do not combine `Agent(isolation: "worktree")` with the codex-rescue forwarder pattern for jobs that change files.** Two workable alternatives:

1. **Direct companion calls without worktrees** (the W3-C/H recovery used this): `node companion.mjs task --background --write` invoked sequentially. Each job shares the working tree, so dispatch must be sequential to avoid index-lock collisions. No worktree, no auto-prune. Reliable.
2. **Pre-create worktrees manually** with `git worktree add` (outside the Agent isolation mode), then dispatch codex from inside them. The manual worktree isn't subject to Agent's auto-prune. Symlink `.claude` into each worktree per memory `feedback_worktree_claude_symlink_for_hooks` so hooks find their scripts.

## Why This Matters

The combination *looks* correct on paper: each parallel task gets its own worktree, isolated from the others, with codex making changes in its own scope. But the Agent tool's "agent didn't change files → prune" heuristic ignores changes made by subprocesses the agent spawned. The pattern silently fails three of four times and leaves no clear error signal — the forwarder agent returns "completed" while the underlying codex process is pointing at a deleted path.

## When to Apply

- Use **direct companion sequential** when codex tasks share a working tree (the W3-C/H recovery pattern). Run 16–26 min wall-clock per phase; the sequential cost is real but the reliability is worth it.
- Use **manual worktrees** when you genuinely need parallel codex jobs touching disjoint files. Per memory `feedback_codex_parallel_3way_unstable`, the parallel ceiling is now 5-way; manual worktrees scale that pattern.
- **Never** use `Agent(isolation: "worktree")` to wrap codex-rescue when the codex job will write files. The combination is a footgun.

## Examples

W3-C/H Phase 3 (DOS-294 schema): direct companion call, 26 min, completed cleanly with full validation output.
W3-C/H Phase 4 (DOS-299 backfill): direct companion call, 16 min, completed cleanly.
W3-C/H Phase 5 (DOS-301 projection): direct companion call, 18 min, completed cleanly.

The W3 retro recorded: *"Switching to direct `node companion.mjs task --background` calls without worktrees works reliably for sequential dispatches."*

## Related

- Memory: `feedback_codex_task_commit_sandbox` (codex `task --write` commit failure pattern)
- Memory: `feedback_codex_parallel_3way_unstable` (5-way max parallel ceiling)
- Memory: `feedback_worktree_claude_symlink_for_hooks` (when manually creating worktrees)
- Memory: `feedback_dense_writes_block_parallel_codex`
