---
title: "Under heavy memory pressure (load>30, swap>70%, many parallel target/ dirs), pre-commit and pre-push cargo gauntlets can hang at 0% CPU — WIP=1 + --no-verify are the documented escape paths when manual cargo test passed clean"
problem_type: tooling_decision
track: knowledge
module: .githooks/pre-commit + .githooks/pre-push (escape-hatch policy)
tags: [pre-commit, pre-push, cargo-test, memory-pressure, disk-pressure, wip-flag, no-verify, ship-velocity]
date: 2026-05-20
related_linear: DOS-741, DOS-742, DOS-743, DOS-576, DOS-577
related_memories: [feedback_disk_pressure_from_worktree_targets, feedback_5min_heartbeat_for_long_jobs]
---

## Context

v1.4.3 W6 session hit aggressive resource pressure with multiple parallel cargo test runs:
- ~6 worktrees with `src-tauri/target/` dirs (each 1.6-7GB; one stale at 21GB)
- 86% disk usage on `/private/tmp`
- Load avg 43
- Swap 80% full (4050M of 5120M used)

In that state, pre-commit and pre-push cargo gauntlets stalled. The cargo wrapper process showed 0% CPU but held 174MB RSS for 15+ min — a zombie waiting on something that wasn't progressing. The actual test runner child process sometimes had bursts of CPU activity (1500%) but couldn't finish.

Multiple sequential commit attempts produced the same hang. Killing one cargo test process didn't always free the next one — the parent shell process held locks.

## The two documented escape paths

### `WIP=1 git commit ...` (pre-commit hook)

From `.githooks/pre-commit` header:

```
# Escape hatches:
#   WIP=1 git commit ...        — skip heavy checks for incremental WIP work
#                                  (.githooks/pre-push runs them at push time)
#   git commit --no-verify      — skip the gate entirely; surface to user
```

`WIP=1` is explicitly named as the right tool for "incremental WIP work" — defers clippy + cargo test + tsc to pre-push, runs cheap checks (PII, stub/TODO, schema sync, ADR-0101 boundary, reference fidelity gate).

### `git push --no-verify` (pre-push hook)

From `.githooks/pre-push` header:

```
# Bypass: git push --no-verify (use sparingly; surface to user).
```

Pre-push has a tree-SHA cache: if pre-commit ran the gauntlet on the same tree SHA, pre-push skips. Under WIP=1, the cache is empty, so pre-push runs cargo test from scratch.

When pre-push itself hangs under memory pressure, `--no-verify` is the documented bypass — but only when manual `cargo test --lib` has already passed clean on the same tree.

## Recovery sequence

1. **Free disk pressure first.** Per memory `feedback_disk_pressure_from_worktree_targets`, sweep at 80% capacity. Stale worktree target dirs are the first victim:
   ```sh
   du -sh /private/tmp/dailyos-*/src-tauri/target
   rm -rf /private/tmp/dailyos-<stale>/src-tauri/target
   ```

2. **Kill zombie cargo.** `ps -ef | grep "cargo test"` — any process at 0% CPU and >10 min elapsed is zombie:
   ```sh
   kill -9 <pid>
   ```

3. **Run manual `cargo test --lib`** in ONE worktree. Don't run parallel cargo tests under memory pressure — they thrash. Wait for it to complete (5-10 min on this codebase).

4. **Commit with `WIP=1`.** Heavy checks deferred to pre-push:
   ```sh
   WIP=1 git commit -F /tmp/msg.txt
   ```

5. **Push with `--no-verify`** IF (a) manual cargo test passed clean on this exact tree AND (b) user has authorized broad shipping. The PR CI re-runs the full gauntlet as the documented backstop.

## When NOT to use these escape hatches

- **`WIP=1` on the final commit of a wave that you intend to ship without further CI gate.** Always push first → CI verifies.
- **`--no-verify` without verified-clean test runs.** The bypass only makes sense when you have independent evidence that tests pass.
- **Either without explicit user authorization** for ship-velocity bypass. "Finish v1.4.3 in this session" counts as broad authorization; a vague task does not.

## Signals that justify the bypass

1. Pre-commit / pre-push cargo test process at 0% CPU for >10 min (zombie state).
2. System load avg >30, swap >70%, disk >85%.
3. Independent `cargo test --lib` runs cleanly on the same tree (manually verified before bypass).
4. User has authorized broad ship-velocity (explicit "ship" / "finish" / "merge" direction).

## Resolution at v1.4.3 W6

Used WIP=1 on PR #337 + #338 commits + --no-verify on both pushes. CI on both PRs re-ran the full gauntlet:
- PR #337: cycle-1 failed (ephemeral refs + HMAC fixture vectors); cycle-2 all green.
- PR #338: passed CI on first try.

Both merged successfully 2026-05-20.

## Don't do this

- **Don't keep retrying `git commit` waiting for cargo test to magically un-hang.** It won't. The hung process consumes memory that prevents the next one from starting.
- **Don't skip the manual `cargo test --lib` step before `--no-verify`.** Without independent test verification, the bypass loses its justification.

## Do this

- **Free disk before retrying.** 21GB freed in this session unblocked the next cargo test.
- **Serialize cargo tests under memory pressure.** Kill duplicates; run one at a time.
- **Document the bypass in the commit message** ("Pre-push gauntlet was bypassed because the system was thrashing under memory pressure... Tests were verified clean independently before push") — surfaces the decision for reviewer + Linear history.

## Cross-references

- Hook docs: `.githooks/pre-commit` + `.githooks/pre-push`
- Memory: `feedback_disk_pressure_from_worktree_targets`, `feedback_5min_heartbeat_for_long_jobs`
- Retro: `.docs/plans/v1.4.3-wp-foundation/retro.md` §"Pre-commit / pre-push gauntlet thrashing under memory pressure"
