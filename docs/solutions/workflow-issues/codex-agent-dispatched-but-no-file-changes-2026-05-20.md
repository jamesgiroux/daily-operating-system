---
title: "Codex-rescue agents return 'task started' / 'completed' without always modifying files — verify git diff before assuming work was done"
problem_type: workflow_issue
track: knowledge
module: codex-rescue subagent dispatch (codex CLI background tasks)
tags: [codex, parallel-execution, agent-verification, ship-velocity, w6, dos-742, dos-743, dos-576]
date: 2026-05-20
related_linear: DOS-742, DOS-743, DOS-576
related_memories: [feedback_codex_returns_early_during_work, feedback_codex_companion_write_flag, feedback_default_to_codex_for_refactor_work]
---

## Context

v1.4.3 W6 needed substrate work on 3 tickets (DOS-742 HMAC signing, DOS-743 MCP invoke audit migrate, DOS-576 11-site migration) plus DOS-741 done manually first. Per memory `feedback_parallel_agent_sizing`, mechanical work + tight rule + non-overlapping files → fan out parallel codex agents.

Dispatched 3 codex-rescue agents in parallel, each with its own worktree branched off `dos-741-request-id-field`:

| Agent | Worktree | Expected work | Files changed |
|---|---|---|---|
| DOS-742 (HMAC signing) | `/private/tmp/dailyos-742` | `surface_runtime/hmac.rs` + `class-dailyos-hmac-signer.php` | **0** |
| DOS-743 (MCP migrate) | `/private/tmp/dailyos-743` | `surface_runtime/mod.rs` 3-branch migration | **0** |
| DOS-576 (11-site migration) | `/private/tmp/dailyos-576-migrate` | commands/* + services/* + CI gate + whitelist | **17 files, 315 insertions** ✓ |

**1 of 3 agents actually executed.** The other 2 returned task-notifications like:

- DOS-742: "The codex task is in progress. It is still running in background ID `bkj134i4v`. I will await completion notification."
- DOS-743: agent returned `completed` with no result text; worktree had zero changes.
- DOS-576: returned a substantive summary AND the worktree had the actual changes.

## Root cause

Codex-rescue agents dispatch a codex CLI task to a background pool. The agent's return shape is "I forwarded this to codex" — not "codex finished and the files were modified." The codex CLI task can:

1. **Succeed** — files modified, agent returns a summary.
2. **Fail silently** — task started, ran, errored, but the agent doesn't see/report the error.
3. **Never start** — task queued, parent agent returned "completed" before codex began.

There's no reliable signal in the agent return text that distinguishes (1) from (2)/(3).

## Signals to detect

After an agent returns "completed" / "task started" / "in progress":

1. **Runtime <60s** for a task expected to take 5-15 min of substantive work.
2. **Return text says "I'll dispatch"** / "task started" / "I'll await completion" without a concrete file-change summary.
3. **`git status` / `git diff --stat` in the target worktree shows zero changes.**

Any of those signals means the work didn't land. Don't assume.

## How to apply

For any codex-rescue agent dispatch where ship-velocity depends on parallel execution:

1. **Set a short polling cadence** to verify file changes. Don't wait for the agent's completion notification — check `git diff --stat` directly in the target worktree.
2. **Have a fallback plan ready.** If 1 of N agents executes, the other N-1 may need manual implementation. Estimate that overhead.
3. **For mechanical work with tight rules, prefer running codex CLI directly** (one-shot, sandbox-mode, verifiable output) over dispatching a rescue agent.

## Resolution at v1.4.3 W6

After detecting that DOS-742 + DOS-743 agents didn't execute:
- Implemented both manually in the DOS-576-migrate worktree (bundling all 4 substrate tickets into one PR).
- DOS-742 (Rust + PHP signer): ~15 min manual.
- DOS-743 (SurfacePairingAuditEvent.request_id thread + 15 construction sites): ~15 min manual.
- Bundled commit: 24 files, 465 insertions. Merged via PR #337.

## Don't do this

- **Don't trust the agent return text alone.** "Task started in background" is not the same as "files modified."
- **Don't wait for completion notification indefinitely.** If the worktree shows no changes after the expected runtime, the work isn't happening.

## Do this

- **Verify with git.** `cd <worktree> && git diff --stat` is the ground truth.
- **Set fallback expectations.** Plan for 1 of N parallel agents possibly failing silently. Buffer the schedule for manual implementation.
- **Pair with [[feedback_codex_returns_early_during_work]]**: codex CLI itself emits "completed" before analysis fully done; the agent layer can amplify this gap.

## Cross-references

- Memory: `feedback_codex_returns_early_during_work`, `feedback_codex_companion_write_flag`
- Retro: `.docs/plans/v1.4.3-wp-foundation/retro.md` §"codex-rescue agent reliability variance"
