---
title: "Pre-push hook duration exceeds GitHub SSH idle timeout — long Rust gauntlet kills the push connection"
problem_type: tooling_decision
track: knowledge
module: .githooks/pre-push, git SSH transport, GitHub remote
component: development_workflow
severity: medium
tags: [pre-push, ssh, github, cargo-test, idle-timeout, rebase, pipestatus]
date: 2026-05-19
last_updated: 2026-05-24
related_linear: DOS-713
applies_when:
  - "Pushing a Rust-touching branch that triggers the full pre-push gauntlet"
  - "Cargo target/ is cold or partially cold (rebuild time is long)"
  - "Pushing immediately after a rebase before the tier-aware gauntlet cache has been warmed for the rebased tree"
  - "Multiple parallel sessions are running pre-push gauntlets simultaneously (resource contention)"
---

## Context

Surfaced during the 2026-05-19 parallel-session fan-out. Session 4 hit this on PR #320 push attempts — deterministic failure, 3 retries on the same commit, identical symptom each time. Session 2 corroborated on PR #323 — same pattern, ~60 min lost across 3 attempts before authorizing the `--no-verify` workaround. The repeatability across independent sessions on the same day is strong evidence this is a structural hook + transport interaction, not a flaky network.

**Historical rebase cache miss.** The old pre-push hook read `<git-dir>/hooks-state/last-green-tree`, which was written only by **pre-commit**. A `git rebase` replays commits without invoking pre-commit, so even byte-identical content was not recorded under the rebased tree SHA. As of 2026-05-24, pre-push writes the shared tier-aware cache after a successful gauntlet, so the first post-rebase push may still pay the cost but same-tree retries do not.

The pre-push hook (`.githooks/pre-push`) on Rust-touching pushes runs:

1. `cargo clippy -- -D warnings` (≈1–2 min warm, longer cold)
2. `cargo test --lib` (5+ min warm, much longer cold)
3. `pnpm tsc --noEmit` (≈30s)

Total: 7–15+ minutes of local work **before** git invokes the SSH transport.

## Guidance

**Don't rely on the SSH connection staying alive through the full gauntlet.** Three mitigations, in increasing cost:

1. **Trust the tier-aware tree cache.** The hook prints `pre-push: HEAD tree <sha> already passed required gauntlet — skipping clippy/test/tsc` when the same tree already passed the required Rust/frontend tier. Retry the push without changing anything — if the cache fires, the gauntlet skips and the push lands. If it doesn't fire on retry, investigate why before assuming the network is at fault.
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

- **Always**: prefer the tier-aware tree-cache path. If the required Rust/frontend tiers already passed on the same tree, the hook should skip them on push.
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

- DOS-713 — originally tracked the tree-cache miss on retry path and optional SSH keepalive for github.com.
- **Resolved 2026-05-24:** `.githooks/pre-commit` and `.githooks/pre-push` now share `<git-dir>/hooks-state/last-green-gauntlet-v2`. The marker records the tree plus which tiers passed (`rust`, `frontend`), so repeated commit attempts and pre-push retries skip duplicate clippy/test/tsc only when the required tier has already passed for that exact tree. Pre-push writes the marker after its own successful gauntlet, closing the rebase-then-retry cache gap without letting doc-only commits bless Rust or frontend changes.
- Related pre-commit improvement: cheap/path-local gates now run before clippy/test/tsc, and heavy checks fail fast with a log path and tail output. A cheap PII/stub/schema/boundary failure should return before the Rust gauntlet starts.
- Related: long pre-push duration is exacerbated when parallel worktrees both invoke the gauntlet at the same time (cargo target/ contention). Coordinate pushes serially across sessions if running the parallel-session protocol.
