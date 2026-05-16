# DOS-440 W7-C L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 7 - Release gate hardening + close.
- **Agent:** W7-C.
- **Linear issue:** DOS-440 - "release-gate: build.rs SHA watching for linked git worktrees" (verbatim in §2 + §5).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. Only file created: `.docs/plans/v1.4.1-waves/W7-C-L0-packet.md`.
- **W7-C assignment:** `src-tauri/build.rs` worktree resolution. Source: `.docs/plans/v1.4.1-waves.md:682-685`.
- **W7 merge gate:** v1.4.1 release-gate close. Source: `.docs/plans/v1.4.1-waves.md:698-712`.
- **Reviewer contract:** qa-expert reviewer on L0 panel. No security-auditor (contributor-setup hardening, not a trust-boundary change).
- **Runtime contract:** `build.rs` runs at compile time and emits `cargo:rerun-if-changed` directives to control rebuild triggers. `BUILD_GIT_SHA` is the env var read at runtime by `release_gate.rs:48`.

## 2. Load-Bearing User Outcome

DOS-440 frames the user-facing failure:

> "`src-tauri/build.rs` emits `cargo:rerun-if-changed=.git/HEAD` (and `.git/<ref>` + `.git/packed-refs`) only. In a linked git worktree, HEAD is per-worktree but normal branch refs and `packed-refs` live under `--git-common-dir`. The current build script never resolves or watches the common dir. In that layout, committing on the current branch leaves Cargo's watched paths untouched, so `build.rs` may not rerun and the release-gate binary keeps embedding the previous SHA."

The load-bearing outcome is: **a contributor working from a linked git worktree (multi-branch development) gets a release-gate binary whose `BUILD_GIT_SHA` actually matches their current HEAD, not a stale value from before the most recent commit.**

Required behavior from DOS-440:

> "Run `git rev-parse --git-common-dir` alongside `--git-dir` at build time. If they differ, the build is in a linked worktree. Emit `cargo:rerun-if-changed` for both: per-worktree HEAD (`--git-dir` + `/HEAD`), common-dir `packed-refs` (`--git-common-dir` + `/packed-refs`), common-dir `refs/heads/<branch>` if HEAD is a symbolic ref pointing to a branch (resolve with `git symbolic-ref`)."

Intelligence Loop fit: none. This is **contributor-environment hardening** for the build pipeline.

## 3. Pre-Work

- **Read W7 source of truth.** `.docs/plans/v1.4.1-waves.md:682-685` assigns W7-C to `src-tauri/build.rs` worktree resolution.
- **Read build.rs.** Currently emits `cargo:rerun-if-changed` for `.git/HEAD`, `.git/<ref>` (if HEAD is symbolic), and `.git/packed-refs`. Does not resolve git-common-dir.
- **Runtime consumer.** `release_gate.rs:48` reads `BUILD_GIT_SHA` env var (set by build.rs). If stale, release-gate fails with `harness-report-stale` or `--git-sha mismatch`.
- **Linked worktree semantics.** `git rev-parse --git-dir` returns the per-worktree git dir (e.g., `.git/worktrees/<name>`); `--git-common-dir` returns the parent repo's `.git` dir. They differ only in linked worktrees.
- **No security-auditor required.** The change is purely about cargo's rebuild-trigger detection; it does not affect trust boundary, sensitivity policy, or claim substrate.

## 4. Architecture

### 4.1 Files Owned

- `src-tauri/build.rs` — extend rerun-if-changed emission for linked worktrees.

### 4.2 Build-Time Resolution

Add to `build.rs`:

```rust
// Resolve both git-dir and git-common-dir.
let git_dir = run_git_rev_parse("--git-dir")?;
let git_common_dir = run_git_rev_parse("--git-common-dir")?;
let in_linked_worktree = git_dir != git_common_dir;
```

If `in_linked_worktree`:
1. Emit `cargo:rerun-if-changed={git_dir}/HEAD` (per-worktree).
2. Emit `cargo:rerun-if-changed={git_common_dir}/packed-refs` (shared refs).
3. If HEAD is symbolic (`git symbolic-ref HEAD` succeeds), resolve to `refs/heads/<branch>` and emit `cargo:rerun-if-changed={git_common_dir}/refs/heads/<branch>`.

If not linked (`git_dir == git_common_dir`): keep existing behavior (no regression for the standard checkout case).

### 4.3 Runtime Smoke Check

Add a smoke check at release-gate startup: `git rev-parse HEAD` and compare to `env!("BUILD_GIT_SHA")`. If mismatch, log a warning (not infra failure — the gate still proceeds, but flags the build as suspect). This is observability, not a new gate.

### 4.4 Manual Repro Documentation

Document the manual repro in `build.rs` comments (no unit test feasible at this scope):

```text
# Manual repro for linked-worktree SHA-watching
git init /tmp/test-repo && cd /tmp/test-repo
git commit --allow-empty -m "first"
git worktree add /tmp/test-wt
cd /tmp/test-wt
# build, note SHA
cargo build --features release-gate -p dailyos
# commit, build again, expect new SHA in BUILD_GIT_SHA
git commit --allow-empty -m "second"
cargo build --features release-gate -p dailyos
```

### 4.5 Intelligence Loop Check

Not applicable — build-time concern, no runtime claim/trust/render impact.

## 5. Acceptance Criteria

DOS-440 Acceptance, quoted verbatim:

> "Run `git rev-parse --git-common-dir` alongside `--git-dir` at build time. If they differ, emit `cargo:rerun-if-changed` for both: per-worktree HEAD + common-dir `packed-refs` + common-dir `refs/heads/<branch>` if HEAD is a symbolic ref pointing to a branch."

Testable decomposition:

1. **Both git-rev-parse paths resolved at build time.** `build.rs` runs both `git rev-parse --git-dir` and `--git-common-dir`.
2. **Linked-worktree detection.** If the two values differ, the linked-worktree codepath fires.
3. **Per-worktree HEAD watched.** `cargo:rerun-if-changed={git_dir}/HEAD` emitted.
4. **Common-dir packed-refs watched.** `cargo:rerun-if-changed={git_common_dir}/packed-refs` emitted.
5. **Common-dir refs/heads/<branch> watched.** If HEAD is symbolic, the branch ref is watched in the common dir, not the per-worktree dir.
6. **Standard checkout regression-free.** For non-linked-worktree builds (the common case), watched paths are unchanged.
7. **Runtime smoke check.** Release-gate logs a warning if `BUILD_GIT_SHA` differs from `git rev-parse HEAD` at runtime.
8. **Manual repro documented.** Comments in `build.rs` include the temp-repo + worktree-add reproduction recipe.
9. **No regression on non-git builds.** If `.git` is absent (source-only), the worktree resolution short-circuits (delegates to W7-D's fail-fast logic).

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-440 supplied verbatim in §2 + §5.
- **Upstream:** none. Can start immediately at W7 wave start.
- **Adjacent:** W7-D (DOS-441 source-only fail-fast) shares the `build.rs` file. Coordinate via single PR or by W7-C landing first + W7-D rebasing on top. Both edit different code paths within `build.rs` so no in-file conflict; the wave can ship them as one combined PR if convenient.
- **Out:** not a release-gate semantic change.

## 7. L0 Reviewer Panel

- **Required reviewer:** `qa-expert`.
- **Review focus:**
  - `git rev-parse --git-common-dir` call is wrapped in error-handling that does not crash the build.
  - Linked-worktree detection is conservative: standard checkouts behave exactly as before.
  - Manual repro recipe is correct (a reviewer should be able to follow it and confirm the SHA changes between builds).
  - Runtime smoke check logs but does not fail the gate.

## 8. L0 Acceptance Gate

L0 passes only if:

1. **Problem fit:** addresses the linked-worktree SHA-staleness, not generic git-dir refactoring.
2. **Conservative default:** non-linked builds unchanged.
3. **Manual repro:** documented in `build.rs` comments.
4. **Runtime smoke check:** logged, not failed.
5. **Reviewer panel:** qa-expert only.

## 9. Out-Of-Scope

- Changing `BUILD_GIT_SHA` consumption in `release_gate.rs` (the env var name + meaning stays identical).
- Adding new build-time env vars beyond `BUILD_GIT_SHA`.
- Source-only fail-fast (that's W7-D).
- Subprocess timeout hardening (that's W7-B).

## 10. Changelog

- **V1 - 2026-05-15:** Initial W7-C L0 packet. Located target file; defined linked-worktree codepath; added runtime smoke check; documented manual repro contract.
