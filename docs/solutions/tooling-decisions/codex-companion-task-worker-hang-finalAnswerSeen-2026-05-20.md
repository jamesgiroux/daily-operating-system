---
title: "codex-rescue subagent hangs indefinitely when codex CLI wedges mid-research — no inactivity timer in companion script"
problem_type: tooling_decision
track: knowledge
module: ~/.claude/plugins/cache/openai-codex/codex/1.0.1/scripts/codex-companion.mjs, codex:codex-rescue subagent
tags: [codex, codex-rescue, parallel-dispatch, sqlite-contention, l0-reviews, l2-reviews]
date: 2026-05-20
related_linear: v1.4.4 W1 L0 review (no Linear ticket; surfaced mid-session)
related_memories: [codex-exec-direct-for-oneshot-reviews, codex-rescue-stuck-after-research-phase, codex-returns-early-during-work]
---

## Context

v1.4.4 W1 L0 review dispatched 4 parallel `codex:codex-rescue` subagents to do adversarial review + substrate-grep consult against L0 packets. All 4 hung at 12:01-12:02 after 5-15 grep commands deep into the research phase. Underlying codex CLI processes had died silently but the worker processes never gave up. Output files frozen mid-research with last `Command completed` lines.

## Root cause

Two layers compose into an unbounded wait:

1. **`codex-companion.mjs task` spawns `task-worker` → `runAppServerTurn`** (in `lib/codex.mjs`). The worker subscribes to codex's stream of events. No inactivity timer exists. `scheduleInferredCompletion` at `lib/codex.mjs:367` only fires AFTER `state.finalAnswerSeen = true`. If codex never emits a final assistant message (e.g., CLI crashed mid-research), the worker waits forever.

2. **`~/.codex/logs_2.sqlite` was 2.7 GB main + 176 MB WAL**. 8 abandoned `codex app-server` daemons from past Claude Code sessions still held file descriptors. 4 parallel codex-rescue dispatches added to the contention. Lock waits stalled codex's session-log writes; some codex processes silently died mid-write.

The combination — silent codex crash + no inactivity timer — produces unbounded worker hang.

## Fix

**Immediate cleanup:**

```sh
# Kill abandoned daemons (verify lsof on logs_2.sqlite first to identify stale ones)
lsof ~/.codex/logs_2.sqlite
kill <stale-pids>

# Compact session log DB
sqlite3 ~/.codex/logs_2.sqlite "PRAGMA wal_checkpoint(TRUNCATE); VACUUM;"
```

**Operational pattern (memory `codex-exec-direct-for-oneshot-reviews`):**

For one-shot codex review/challenge/consult on docs or plans, **do NOT use `codex:codex-rescue`**. Use `codex exec` direct via Bash:

```sh
timeout 600 codex exec --skip-git-repo-check -C <worktree> '<prompt>' < /dev/null
```

`/codex:review` and `/codex:adversarial-review` (codex-companion's `review` / `adversarial-review` subcommands) target git diffs, NOT the task-worker hang path — they're reliable.

Reserve `codex:codex-rescue` for genuine multi-turn coding tasks where session continuity (`--resume-last`) matters.

## Verification

After cleanup on 2026-05-20: DB shrank from 2.7 GB → 1.7 GB; WAL drained 176 MB → 0 B; 4 stale daemons killed. Subsequent codex review dispatches via `codex-companion.mjs review` (fast path) and `codex exec` direct completed reliably in cycles 2 and 3.

## Open follow-up

Patching `lib/codex.mjs:297-385` to add an inactivity timer in `createTurnCaptureState` would close the structural gap. Would need to land in the openai-codex plugin upstream OR maintained as a local override. Not yet filed.
