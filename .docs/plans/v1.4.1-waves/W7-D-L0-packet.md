# DOS-441 W7-D L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 7 - Release gate hardening + close.
- **Agent:** W7-D.
- **Linear issue:** DOS-441 - "release-gate: build.rs fail-fast for source-only no-git builds with release-gate feature" (verbatim in §2 + §5).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. Only file created: `.docs/plans/v1.4.1-waves/W7-D-L0-packet.md`.
- **W7-D assignment:** `src-tauri/build.rs` source-only branch + `DAILYOS_BUILD_SHA` opt-in. Source: `.docs/plans/v1.4.1-waves.md:687-690`.
- **W7 merge gate:** v1.4.1 release-gate close. Source: `.docs/plans/v1.4.1-waves.md:698-712`.
- **Reviewer contract:** qa-expert reviewer on L0 panel. No security-auditor (build-time contributor-environment hardening).

## 2. Load-Bearing User Outcome

DOS-441 frames the user-facing failure:

> "When `DAILYOS_BUILD_SHA`, `GITHUB_SHA`, and `git rev-parse HEAD` are all unavailable, `src-tauri/build.rs` emits `BUILD_GIT_SHA=unknown` and the build succeeds. Runtime then rejects that value with infra failure, so a CI tarball or source-only build without `.git` produces a binary that compiles but every release-gate invocation is unusable."

The load-bearing outcome is: **a build without any SHA source fails at compile time with a clear error message, not at runtime with a confusing infra failure on every gate invocation.**

Required behavior from DOS-441:

> "Make `build.rs` FAIL with a clear error when all three SHA sources are unavailable AND the `release-gate` feature is enabled. Detect feature via `CARGO_FEATURE_RELEASE_GATE` env. Error message: `\"BUILD_GIT_SHA cannot be determined. Set DAILYOS_BUILD_SHA, GITHUB_SHA, or run inside a git checkout. For source-only local builds, set DAILYOS_BUILD_SHA=dev-unknown.\"`. Allow explicit opt-in: if `DAILYOS_BUILD_SHA` is set to anything including `dev-unknown`, use that value as-is. Runtime rejects `dev-unknown` the same way it rejects `unknown` — but the build at least succeeds with intent visible."

Intelligence Loop fit: none. Build-time hardening.

## 3. Pre-Work

- **Read W7 source of truth.** `.docs/plans/v1.4.1-waves.md:687-690` assigns W7-D to `src-tauri/build.rs` source-only fail-fast.
- **Read DOS-441 ticket text.** Above quotes in §2.
- **Coordinate with W7-C.** W7-C (DOS-440) and W7-D (DOS-441) both edit `src-tauri/build.rs`. They touch different code paths (W7-C: linked-worktree resolution; W7-D: fail-fast + DAILYOS_BUILD_SHA opt-in). Coordinate via single PR with both changes or land in sequence.
- **Cargo feature detection.** Cargo sets `CARGO_FEATURE_<NAME>=1` env var at build time when a feature is active. `CARGO_FEATURE_RELEASE_GATE` is the detection key.
- **DAILYOS_BUILD_SHA semantics.** Currently optional env var that overrides SHA detection. Extend to "anything goes" — including `dev-unknown` — so source-only builds can explicitly opt out.
- **Runtime gate behavior unchanged.** `release_gate.rs` continues to reject `unknown` / `dev-unknown` SHAs as infra failures. The W7-D change is build-time only.

## 4. Architecture

### 4.1 Files Owned

- `src-tauri/build.rs` — add fail-fast branch when no SHA source available AND feature enabled.

### 4.2 Fail-Fast Logic

Pseudo-code for the relevant `build.rs` branch:

```rust
let dailyos_build_sha = std::env::var("DAILYOS_BUILD_SHA").ok();
let github_sha = std::env::var("GITHUB_SHA").ok();
let git_rev_parse_head = run_git_rev_parse_head(); // existing helper

let release_gate_enabled = std::env::var("CARGO_FEATURE_RELEASE_GATE").is_ok();

let sha = match (dailyos_build_sha, github_sha, git_rev_parse_head) {
    (Some(v), _, _) => v,           // explicit override, always wins
    (_, Some(v), _) => v,           // CI provides
    (_, _, Some(v)) => v,           // git resolves
    (None, None, None) if release_gate_enabled => {
        // FAIL the build with a clear message
        panic!(
            "BUILD_GIT_SHA cannot be determined. \
             Set DAILYOS_BUILD_SHA, GITHUB_SHA, or run inside a git checkout. \
             For source-only local builds, set DAILYOS_BUILD_SHA=dev-unknown."
        );
    }
    (None, None, None) => "unknown".to_string(),  // feature disabled, keep current behavior
};
```

### 4.3 Feature-Gated Strictness

Only fail when `CARGO_FEATURE_RELEASE_GATE` is set. Without the feature, keep the existing `BUILD_GIT_SHA=unknown` fallback so non-release-gate builds (frontend dev, tauri preview, etc.) continue to work without a `.git` directory.

### 4.4 Explicit Opt-In Path

`DAILYOS_BUILD_SHA=dev-unknown` is an explicit no-git opt-in. It satisfies the build but the runtime gate rejects the value at gate invocation. This makes the failure visible at gate time (clear error message) instead of an inscrutable infra failure with no context.

### 4.5 Coordination With W7-C

`build.rs` also gets W7-C's linked-worktree resolution change. The two edits sit in different branches of the `build.rs` logic flow (W7-C: which paths to watch; W7-D: which SHA source to use). No conflict in practice; PR ordering matters only for clean diff hygiene.

### 4.6 Intelligence Loop Check

Not applicable — build-time concern.

## 5. Acceptance Criteria

DOS-441 Acceptance, quoted verbatim:

> "Make `build.rs` FAIL with a clear error when all three SHA sources are unavailable AND the `release-gate` feature is enabled. Allow explicit opt-in: if `DAILYOS_BUILD_SHA` is set to anything (including `dev-unknown`), use that value as-is."

Testable decomposition:

1. **Fail-fast on no-source + feature-enabled.** Build with no `.git`, no `DAILYOS_BUILD_SHA`, no `GITHUB_SHA`, `release-gate` feature → compile error with the named message.
2. **No fail when feature disabled.** Build with no SHA sources but feature OFF → succeeds with `BUILD_GIT_SHA=unknown` (existing behavior).
3. **`DAILYOS_BUILD_SHA` opt-in works for `dev-unknown`.** Setting `DAILYOS_BUILD_SHA=dev-unknown` allows source-only builds to compile under feature-enabled mode. Runtime gate still rejects `dev-unknown` (separate concern).
4. **`GITHUB_SHA` works.** CI sets `GITHUB_SHA`; build succeeds with that value.
5. **`git rev-parse HEAD` works.** Normal git checkout; build succeeds with the actual SHA.
6. **Precedence order:** `DAILYOS_BUILD_SHA` > `GITHUB_SHA` > `git rev-parse HEAD`.
7. **Error message exact.** The panic message matches the DOS-441 specification verbatim.
8. **Regression test (manual repro documented).** `build.rs` comments include manual repro for each branch: explicit env, CI env, git checkout, source-only fail, source-only opt-in.

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-441 supplied verbatim in §2 + §5.
- **Upstream:** none.
- **Adjacent:** W7-C (DOS-440 linked-worktree SHA-watching). Same file. Either coordinate via single PR with both changes, or land in sequence with rebase. No in-file conflict in practice.
- **Out:** not a runtime gate change.

## 7. L0 Reviewer Panel

- **Required reviewer:** `qa-expert`.
- **Review focus:**
  - Panic message matches the ticket verbatim.
  - Feature detection via `CARGO_FEATURE_RELEASE_GATE` is correct (Cargo sets this; verify the case-sensitive variant).
  - Source-only builds without feature continue to work.
  - Precedence order documented + asserted.
  - Manual repro documented in `build.rs` comments.

## 8. L0 Acceptance Gate

L0 passes only if:

1. **Problem fit:** addresses no-SHA-source build failure at compile time when feature is on.
2. **Feature-gated:** only fires when `CARGO_FEATURE_RELEASE_GATE` is set.
3. **Opt-in path:** `DAILYOS_BUILD_SHA=dev-unknown` works.
4. **No regression for non-release-gate builds:** existing fallback preserved.
5. **Error message:** verbatim from ticket.
6. **Reviewer panel:** qa-expert only.

## 9. Out-Of-Scope

- Changing the runtime gate's rejection of `unknown` / `dev-unknown` values.
- Adding new SHA sources beyond the three named.
- Linked-worktree SHA-watching (that's W7-C).
- Subprocess timeout hardening (that's W7-B).

## 10. Changelog

- **V1 - 2026-05-15:** Initial W7-D L0 packet. Defined fail-fast branch + DAILYOS_BUILD_SHA opt-in + feature detection + coordination with W7-C.
