# DOS-439 W7-B L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 7 - Release gate hardening + close.
- **Agent:** W7-B.
- **Linear issue:** DOS-439 - "release-gate: timeout + bounded output capture in run_dos288_selector" (verbatim content in §2 + §5).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. Only file created: `.docs/plans/v1.4.1-waves/W7-B-L0-packet.md`.
- **W7-B assignment:** `release_gate.rs::run_dos288_selector` timeout machinery. Source: `.docs/plans/v1.4.1-waves.md:677-680`.
- **W7 merge gate:** v1.4.1 release gate (different from W6 wave gate). Source: `.docs/plans/v1.4.1-waves.md:698-712`.
- **Reviewer contract:** qa-expert reviewer on L0 panel. The wave plan does not require security-auditor for this agent because the change is subprocess-hardening, not a new trust boundary. Source: `.docs/plans/v1.4.1-waves.md:700-712`.
- **Runtime contract:** release-gate is a stand-alone Rust binary in the `dailyos` workspace. The selector subprocess runs `cargo test` with `--include-ignored --features release-gate -- dos288_*` filter. Source: search of `src-tauri/src/release_gate.rs` for `run_dos288_selector`.

## 2. Load-Bearing User Outcome

DOS-439 frames the user-facing failure:

> "`run_dos288_selector` in `src-tauri/src/release_gate.rs` shells out to `cargo test` with `Command::new(...).output()` and no timeout, kill-on-expiry, or bounded stdout/stderr capture. If the selector deadlocks or emits unbounded `--nocapture` output, the release gate never records an infra failure and CI sits until the outer job times out."

The load-bearing outcome is therefore: **the release gate fails fast and visibly when the selector misbehaves, instead of being absorbed by the outer CI job's timeout.**

Required behavior from DOS-439:

> "Use `std::env::var(\"CARGO\").unwrap_or_else(|_| \"cargo\".to_string())` for the binary path. Spawn via `Command::new(...).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()` not `.output()`. Wrap with configurable timeout (default 600s for the bleed selector) using `std::sync::mpsc::channel` + thread + `child.wait_with_output()` timeout. On expiry: `child.kill()`, return `GateStatus::InfraFailure` with failure summary `dos288-selector-timeout-exceeded`. Cap captured stdout/stderr to 64KB each. Plumb timeout through `GateConfig` as a field with sane default; CLI override allowed for tests, not production."

Intelligence Loop fit: this is **infrastructure hardening**. No claim model / provenance / signal changes. The bounded behavior of the release-gate binary is what the wave needs.

## 3. Pre-Work

- **Read W7 source of truth.** `.docs/plans/v1.4.1-waves.md:677-680` assigns W7-B to `release_gate.rs::run_dos288_selector` timeout machinery.
- **Read W7 merge gate.** v1.4.1 release-gate close requires `pnpm release-gate -- --mode hermetic` exit zero against bundles 1-18 + manual dogfood evidence ≥20 meetings + proof bundle + tag on `trunk`. Source: `.docs/plans/v1.4.1-waves.md:707-712`.
- **Located target function.** `src-tauri/src/release_gate.rs::run_dos288_selector` currently shells `cargo test` via `Command::new("cargo").args([...]).output()`. The DOS288_SELECTORS const at `release_gate.rs:59-62` names the bleed-detection + ownership-validator selectors.
- **Identified config surface.** `GateConfig` struct at `release_gate.rs:71-83` — add `dos288_timeout_secs: u64` (or `Duration`) field with default 600s.
- **Identified evidence surface.** `GateStatus::InfraFailure` + `SuiteResult.failure_summary` at `release_gate.rs:113-141` carry the failure mode through to the evidence report.
- **No security-auditor required.** Subprocess hardening adds resilience but does not change trust boundary, sensitivity policy, or claim render. Wave plan's L0 panel matches (qa-expert only).
- **Source for DOS-288 selectors.** `DOS288_SELECTORS: &[&str] = &["dos288_bleed_detection_test", "dos288_ownership_validator_test"]` at `release_gate.rs:59-62`. The 600s default applies to both.
- **CI runtime context.** `pnpm release-gate -- --mode hermetic` invokes the release-gate binary which calls `run_dos288_selector`. Outer CI job timeout (typically 60-90 min) is the current backstop.

## 4. Architecture

### 4.1 Files Owned

- `src-tauri/src/release_gate.rs` — modify `run_dos288_selector` + `GateConfig`.
- `src-tauri/tests/release_gate_dos288_subprocess_timeout_test.rs` — new regression test.

### 4.2 GateConfig Extension

Add field to `GateConfig` (`release_gate.rs:71-83`):

```rust
pub struct GateConfig {
    // ... existing fields ...
    pub dos288_timeout_secs: u64,
}
```

Default: 600 (10 minutes, generous for the bleed-detection workload). CLI override via clap with `#[arg(long, default_value = "600")]`. Production callers do not override; the override exists for the regression test only.

### 4.3 run_dos288_selector Rewrite

Replace `.output()` call with:

1. Resolve cargo binary: `let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());`
2. Build command with `Command::new(&cargo).args([...]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?`
3. Spawn a thread that calls `child.wait_with_output()` and sends the result on an `mpsc::channel`.
4. Main thread does `rx.recv_timeout(Duration::from_secs(config.dos288_timeout_secs))`.
5. On `RecvTimeoutError::Timeout`: `child.kill()` (best-effort), return `GateStatus::InfraFailure` with `failure_summary = "dos288-selector-timeout-exceeded"`.
6. On `Ok(output)`: process normally. Truncate stdout + stderr to 64KB each with explicit `[... truncated, N bytes total ...]` marker.

### 4.4 Bounded Output

64KB cap on each of stdout and stderr. If captured bytes exceed cap, retain first 32KB + last 16KB with a `[truncated]` separator. Total preserved per stream: 48KB + marker. Marker format: `\n[... 14 bytes preserved at head; M bytes truncated; 16384 bytes preserved at tail ...]\n`.

### 4.5 Failure Summary Vocabulary

New named summary value: `dos288-selector-timeout-exceeded`. Distinguishes timeout from selector-test-failures (which are mandatory bundle failures, not infra failures).

### 4.6 Intelligence Loop Check

- **Claim model:** no change.
- **Provenance and trust:** no change.
- **Signals and invalidation:** no change.
- **Runtime and surfaces:** release-gate binary behavior change only; no MCP / Tauri surface impact.
- **Feedback loop:** no change.

This packet has no Intelligence Loop fit by design — it is CI infrastructure hardening, not a claim-substrate feature.

## 5. Acceptance Criteria

DOS-439 Acceptance, quoted verbatim:

> "Use `std::env::var(\"CARGO\").unwrap_or_else(|_| \"cargo\".to_string())` for the binary path. Spawn via `Command::new(...).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()`. Wrap with a configurable timeout. On expiry: `child.kill()`, return `GateStatus::InfraFailure` with failure summary `dos288-selector-timeout-exceeded`. Cap captured stdout/stderr to 64KB each — truncate with marker if larger. Plumb timeout through `GateConfig`."

Testable decomposition:

1. **`$CARGO` env honored.** Setting `CARGO=/path/to/wrapper` causes the spawn to invoke `/path/to/wrapper test ...` instead of relying on PATH.
2. **Subprocess uses `spawn()` not `output()`.** The change-set replaces the blocking `.output()` call with a thread + channel timeout pattern.
3. **Timeout configurable.** `GateConfig.dos288_timeout_secs` exists with default 600.
4. **On expiry, child killed + `GateStatus::InfraFailure` returned.** Regression test asserts evidence carries status=fail + failure_summary="dos288-selector-timeout-exceeded".
5. **Bounded output.** Stdout > 64KB and stderr > 64KB each get truncated with explicit marker.
6. **Existing tests unaffected.** Suite S/P/E + bundle gates still green; only the selector pathway changes.
7. **Regression test green.** `tests/release_gate_dos288_subprocess_timeout_test.rs` uses fake-cargo wrapper (sleeping shell script) via `$CARGO` env override; asserts timeout-specific failure summary.
8. **CLI default conservative.** Production callers do not override; the 600s default is documented as the bleed-selector budget.
9. **No bypass paths.** The kill-on-expiry path cannot leak a zombie process for >100ms (best-effort but checked).
10. **Evidence report integrity.** When the timeout fires, the evidence JSON's `suites[].failure_summary` contains the named token; downstream consumers can pattern-match.

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-439 supplied verbatim in §2 + §5.
- **Upstream:** none. W7-B can start at the W7 wave start (which begins on W6 merge — completed).
- **Adjacent W7 coordination:** W7-C (DOS-440 build.rs worktree-aware SHA) + W7-D (DOS-441 build.rs source-only fail-fast) both touch the release-gate binary's contributor ergonomics. No file overlap with W7-B.
- **Out:** not a release-gate semantic change; same gate verdicts as before for green paths, new behavior on timeout-or-runaway-output paths only.

## 7. L0 Reviewer Panel

- **Required reviewer:** `qa-expert`.
- **Panel reason:** Wave plan §700-712 names L0 → L2 → L3 → L4 → L5 review ladder; per W7 default and the absence of trust-boundary changes, qa-expert is the only required L0 reviewer.
- **Security reviewer:** not required. The change does not touch sensitivity, render policy, MCP, or claim substrate.
- **Review focus for `qa-expert`:**
  - Regression test reproducibly fires the timeout path.
  - 64KB cap actually truncates (test with > 64KB stdout).
  - `$CARGO` env override works (test with fake-cargo wrapper).
  - Failure summary token is stable and matches downstream consumers' pattern-match.
  - Zombie processes cannot leak after kill (best-effort process cleanup).

## 8. L0 Acceptance Gate

L0 passes only if reviewer accepts all of the following:

1. **Problem fit:** the plan addresses the deadlock-or-runaway path, not generic subprocess refactoring.
2. **Config plumbing:** `GateConfig.dos288_timeout_secs` is the single source of truth; no hardcoded literal in the subprocess code.
3. **Failure summary:** named token `dos288-selector-timeout-exceeded` is in evidence on timeout.
4. **Test approach:** regression test uses `$CARGO` env override + sleeping shell script; no real `cargo test` invocation in the test.
5. **Bounded output:** 64KB cap with truncation marker.
6. **Reviewer panel:** qa-expert only.
7. **No PII:** test fixtures are synthetic.

## 9. Out-Of-Scope

- Changing the selector test set itself (DOS288_SELECTORS).
- Refactoring other subprocess call sites in release_gate.rs that don't run `cargo test`.
- Adding a global subprocess-timeout policy across all gate steps.
- Changing the green-path output volume (only the truncation path is bounded).
- Build.rs SHA-watching work (that's W7-C/D).

## 10. Changelog

- **V1 - 2026-05-15:** Initial W7-B L0 packet. Located target function + GateConfig surface; locked failure-summary token; named qa-expert as sole L0 reviewer; defined 64KB truncation contract.
