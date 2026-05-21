---
title: "codex-rescue agent runs `git reset --hard public/dev` mid-failure, wiping uncommitted work"
problem_type: workflow_issue
track: knowledge
module: codex:codex-rescue subagent, .claude/plugins/data/codex-openai-codex/lib/codex.mjs
tags: [codex, codex-rescue, destructive-action, git-reset, 529-overloaded, sandboxing, sub-agent-safety]
date: 2026-05-20
related_linear: DOS-746
related_memories: [codex-rescue-destructive-git-reset-on-failure, codex-exec-direct-for-oneshot-reviews, dont-rm-claude-without-verify]
---

## Context

While shipping DOS-746 (pairing scope refresh), the orchestrator dispatched two `codex:codex-rescue` agents back-to-back with `--write` access. Both failed with Anthropic API `529 Overloaded`. After each failure, the orchestrator found:

- Original branch HEAD reset to `public/dev` (visible via `git reflog`: `HEAD@{0}: reset: moving to public/dev`)
- ~250 LOC of uncommitted Rust + PHP work wiped from the worktree
- Only untracked files (test files added with `Write`) survived; everything that had been staged or modified-tracked was gone

Recovery required re-applying the entire diff from the conversation context (twice in one session).

## Root cause

The codex-rescue agent has `--write` access, which includes the ability to run arbitrary git commands. On 529 failure during execution, the agent's error path (in `.claude/plugins/data/codex-openai-codex/lib/codex.mjs` or equivalent) appears to attempt a "clean state" recovery via `git reset --hard <baseline>`. There is no inactivity timer guarding this — when codex wedges, the recovery path runs unconditionally.

The system reminders about "intentional change" that appear in Claude Code after such resets point at the destination state of the reset, not at the deletion itself, so the orchestrator doesn't get a clear signal that work was lost until it checks `git status` and finds the working tree clean.

## Fix

Three layers of defense, in order of preference:

1. **Commit before dispatching** any `--write`-capable codex agent when there is uncommitted substrate work in the worktree. Even a checkpoint commit with `L2-status: not-run-acknowledged` is fine — recoverable, doesn't destroy work.

2. **Prefer `isolation: "worktree"`** when spawning codex-rescue with `--write`. The agent gets a fresh worktree it can corrupt freely; the main worktree is untouched. This is what eventually worked for DOS-575 (PR #343).

3. **For one-shot reviews, switch to `codex exec` direct** via `Bash` with explicit `timeout`. Pattern that worked this session:

   ```bash
   timeout 2700 codex exec --json --full-auto --skip-git-repo-check \
     < /tmp/prompt.txt > /tmp/codex-exec.log 2>&1
   ```

   `codex exec` runs in the current cwd, has predictable lifecycle (timeout kills it cleanly), and does NOT auto-reset on failure.

## Defensive checks

After any codex-rescue dispatch that returns failure (especially 529), check immediately:

```bash
git reflog | head -5
```

If you see `reset: moving to <branch>` at `HEAD@{0}`, the agent destroyed your work. Recovery is from conversation context; the staged-but-not-committed changes are gone from disk (though they may persist as dangling blobs in `git fsck --lost-found` for a short window).

## Prevention checklist

Before dispatching ANY codex-rescue or other `--write`-capable agent:

- [ ] `git status` shows clean working tree, OR
- [ ] Uncommitted changes are committed (checkpoint commit is fine), OR
- [ ] Dispatch uses `isolation: "worktree"`

For L2 / adversarial review work specifically, prefer `codex exec` direct over `codex-rescue` — see [codex-exec-direct-for-oneshot-reviews](../../../.../) memory.

## Related

- Memory `feedback_codex_rescue_destructive_git_reset_on_failure` — original capture, persists across sessions.
- Memory `feedback_codex_exec_direct_for_oneshot_reviews` — recommended alternate path.
- Analogous: `feedback_dont_rm_claude_without_verify` — same lesson applied to `.claude` removal.
