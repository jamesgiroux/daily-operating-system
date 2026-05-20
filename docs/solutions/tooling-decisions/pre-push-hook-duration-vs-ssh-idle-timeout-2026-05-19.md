---
title: "Pre-push hook duration exceeds GitHub SSH idle timeout — long Rust gauntlet kills the push connection"
problem_type: tooling_decision
track: knowledge
module: .githooks/pre-push, git SSH transport, GitHub remote
component: development_workflow
severity: medium
tags: [pre-push, ssh, github, cargo-test, idle-timeout, rebase, pipestatus]
date: 2026-05-19
last_updated: 2026-05-19
related_linear: DOS-713
applies_when:
  - "Pushing a Rust-touching branch that triggers the full pre-push gauntlet"
  - "Cargo target/ is cold or partially cold (rebuild time is long)"
  - "Pushing immediately after a rebase (rebased commits never trigger pre-commit, so the last-green-tree cache is always cold)"
  - "Multiple parallel sessions are running pre-push gauntlets simultaneously (resource contention)"
---

## Context

Surfaced during the 2026-05-19 parallel-session fan-out. Session 4 hit this on PR #320 push attempts — deterministic failure, 3 retries on the same commit, identical symptom each time. Session 2 corroborated on PR #323 — same pattern, ~60 min lost across 3 attempts before authorizing the `--no-verify` workaround. The repeatability across independent sessions on the same day is strong evidence this is a structural hook + transport interaction, not a flaky network.

**Why the cache always misses on rebase-then-push.** The pre-push hook reads `<git-dir>/hooks-state/last-green-tree` and skips the gauntlet when HEAD's tree matches. That cache file is written only by **pre-commit**, not by pre-push. A `git rebase` replays commits without invoking pre-commit, so even though the file contents are byte-identical to the pre-rebase commit, the new commit object's tree-SHA hasn't been recorded in the cache. Every post-rebase push pays the full gauntlet cost.

The pre-push hook (`.githooks/pre-push`) on Rust-touching pushes runs:

1. `cargo clippy -- -D warnings` (≈1–2 min warm, longer cold)
2. `cargo test --lib` (5+ min warm, much longer cold)
3. `pnpm tsc --noEmit` (≈30s)

Total: 7–15+ minutes of local work **before** git invokes the SSH transport.

## Guidance

**Don't rely on the SSH connection staying alive through the full gauntlet.** Three mitigations, in increasing cost:

1. **Trust the tree-SHA cache.** The hook already prints `pre-push: HEAD tree <sha> already passed pre-commit gauntlet — skipping clippy/test/tsc` when the same tree was pre-commit-validated earlier. Retry the push without changing anything — if the cache fires, the gauntlet skips and the push lands. If it doesn't fire on retry (e.g., after a rebase changed the tree SHA), investigate why before assuming the network is at fault.
2. **Pre-validate the gauntlet locally, then push with `--no-verify`** when you've already manually run clippy + test + tsc and have explicit authorization (per CLAUDE.md "Never skip hooks unless the user explicitly requests it"). The hook is duplicating work you've already done.
3. **Configure SSH keepalive** for the `github.com` host (`ServerAliveInterval 60` in `~/.ssh/config`). Lower-friction long-term fix.

## Why this matters

GitHub's SSH server has an idle-connection timeout (exact threshold not published; observed to be in the 5–10 min range under load). The pre-push hook runs **after** git opens the SSH connection to negotiate refs but **before** it sends pack data. If the hook takes longer than the idle timeout, GitHub kills the connection. The hook completes successfully, git tries to push, and you see:

```
pre-push: cargo clippy -- -D warnings
pre-push: cargo test --lib
Connection to github.com closed by remote host.
pre-push: pnpm tsc --noEmit
```

The hook gates aren't catching anything — `ssh -T git@github.com` in isolation succeeds, all three gates pass when run manually. The network is the blocker, not the code. Without keepalive or `--no-verify`, the push is deterministically broken on long-gauntlet commits.

## When to apply

- **Always**: prefer the tree-SHA cache path. If you already ran clippy + test + tsc locally, the hook should skip them on push.
- **As a fallback** when the cache misses and you've manually validated: `--no-verify` is justified, but only after surfacing the call to the user per CLAUDE.md.
- **Long-term**: file a Maintenance ticket to investigate either the cache-miss path or SSH keepalive. DOS-713 covers both.

## Examples

**Symptom:**

```
$ git push --force-with-lease public my-branch
pre-push: HEAD tree abc12345 — running clippy/test/tsc gauntlet
pre-push: cargo clippy -- -D warnings
   Compiling …
pre-push: cargo test --lib
   Running unittests …
Connection to github.com closed by remote host.
pre-push: pnpm tsc --noEmit
   Finished
error: failed to push some refs to 'github.com:…'
```

Three retries → same pattern, deterministic failure.

**Workaround used in PR #320:**

```bash
# After manually validating gates pass:
cargo clippy --workspace -- -D warnings     # ✓
cargo test --lib --workspace claim_type     # ✓
pnpm tsc --noEmit                            # ✓

# Authorized one-off bypass:
git push --force-with-lease --no-verify public v1.4.6-w0-adr-0125
# ✓ push lands in seconds
```

**Diagnostic — get the real exit code (PR #323).** Several push retries in session 2 looked clean because the wrapper was `git push 2>&1 | tail -15 ; echo $?`. The `$?` there reports `tail`'s exit, not git push's, and `tail` always succeeds — so the real `141` (SIGPIPE) was hidden. Use `PIPESTATUS` to recover it:

```bash
# WRONG — reports tail's exit code, hides push's 141
git push 2>&1 | tail -15 ; echo $?

# RIGHT — PIPESTATUS array preserves each stage
git push 2>&1 | tail -15 ; echo "push_exit=${PIPESTATUS[0]}"
```

A `push_exit=141` after the gauntlet's clippy + cargo test + tsc lines all print without error is the canonical signature of this issue.

**Verbose trace recipe** (for first diagnosis on a new repo or transport):

```bash
GIT_TRACE=1 GIT_SSH_COMMAND='ssh -v' git push 2>&1 | tee /tmp/push.log
echo "push_exit=${PIPESTATUS[0]}"
grep -E "ssh.*disconnect|broken pipe|EOF|closed by remote" /tmp/push.log
```

## Tracking

- DOS-713 — investigate tree-SHA cache miss on retry path; optionally configure SSH keepalive for github.com.
- **Structural fix candidate**: teach `.githooks/pre-push` to write `<git-dir>/hooks-state/last-green-tree` on its own successful gauntlet pass, not just pre-commit. That closes the rebase-then-push cache gap permanently — after one expensive gauntlet run on the rebased tree, subsequent pushes from the same tree (e.g., retry, force-push of identical content) short-circuit in <1s instead of re-running the 17-min suite. Approximate one-liner to append to the hook's success path:
  ```bash
  git_dir="$(git rev-parse --git-dir)"
  mkdir -p "$git_dir/hooks-state"
  git rev-parse HEAD^{tree} > "$git_dir/hooks-state/last-green-tree"
  ```
- Related: long pre-push duration is exacerbated when parallel worktrees both invoke the gauntlet at the same time (cargo target/ contention). Coordinate pushes serially across sessions if running the parallel-session protocol.
