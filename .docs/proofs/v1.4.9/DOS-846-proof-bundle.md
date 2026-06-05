# DOS-846 Proof Bundle - Guarded Claude Desktop MCP Runtime

**Date:** 2026-06-03; L2 remediation updated 2026-06-05
**Branch:** `codex/v1.4.9-w1-dos846`
**Issue:** [DOS-846](https://linear.app/a8c/issue/DOS-846)
**Original validation build SHA:** `2584b718b8cb2227756c6ffe4efe44577d481048`
**Current L2 remediation build SHA:** `ab81de9c7b30f591c450e714bcb353dfe70a40ea+dirty.74d024cd7642`

The sidecar build SHA above is the original validation build commit. L2
remediation on 2026-06-04 added atomic managed-runtime replacement, strict
launcher/config argument and env validation, source-kind/DB-mode and target-triple
invariants, expanded refusal-matrix tests, local preflight parity, build-script
provenance staging, packaged app-bundle hash handling for signed sidecar bytes,
app-bundle signature verification before trusting final signed hashes, self-check
pipe timeout hardening, read-only status validation, launch-time AppBundle
signature/root enforcement in the launcher, serialized configure transactions,
atomic JSON writes, bounded signature checks, process-group cleanup, AppBundle
identity pinning, outer timeout budgeting, an inter-process configure lock,
versioned runtime generations, malformed-config fail-closed behavior, managed
launcher tamper rejection, launcher-check diagnostics, no-env launcher
execution, AppBundle trusted-anchor verification, generation-based build
publishing, current-symlink replacement with `os.replace`, and cleanup ordering
that preserves a generation immediately after activation.

## Scope

DOS-846 closes the stale Claude Desktop MCP binary gap by moving Claude config to an app-managed launcher, verifying bundled build provenance before execution, running a bounded no-DB self-check, and pinning explicit DB mode for final MCP startup.

This work does not add a schema migration, MCP tool, claim field, or user-facing intelligence surface.

## Acceptance Evidence

| AC | Evidence |
| --- | --- |
| AC1 | `configure_claude_desktop()` now writes `mcpServers.dailyos.command` to the app-managed launcher with `--manifest`; focused tests verify raw `target/debug`, `target/release`, and `~/.cargo/bin` config entries are rewritten. |
| AC2 | `get_claude_desktop_status()` now marks raw commands, duplicate/unknown launcher args, env overrides, missing launchers, non-executable launchers/sidecars, stale manifests, wrong hashes, missing sidecars, source-kind/DB-mode mismatches, and failed or timed-out launcher checks as unsafe. Status validation is read-only and does not chmod runtimes. |
| AC3 | `build-mcp.sh`, `Cargo.toml`, and `tauri.conf.json` build/package `dailyos-mcp`, `dailyos-mcp-launcher`, and provenance JSON. Synthetic app-bundle proof verifies packaged source-kind path semantics and signed-byte hash drift between build provenance and final app-bundle files; production app-bundle resolution requires a valid signed `.app` before final hashes are recorded, and the launcher re-verifies AppBundle root/signature plus DailyOS bundle identifier and TeamIdentifier before granting `live` DB mode. |
| AC4 | Launcher tests cover missing provenance, stub provenance, wrong target triple, build SHA mismatch, moved launcher, duplicate args, missing launcher/sidecar, wrong source-kind path, wrong hash, zero-byte sidecar, DB-mode/source-kind mismatch, repo-binaries manifest/provenance mismatch before spawn, app-bundle manifests whose final signed hashes differ from build provenance, forged unsigned app-bundle manifests, sidecars outside the provenance bundle root, wrong TeamIdentifier, and wrong bundle identifier. Integration tests reject stub provenance, wrong target provenance, stale manifests, duplicate manifest args, env overrides, repo-binaries/live DB mode, unsigned packaged app bundles when signature verification is enabled, non-executable managed runtimes without chmod, and concurrent configure races. |
| AC5 | Launcher validates `--self-check-json` guard epoch, build SHA, no-env `replica` default, guard presence, `dbOpened:false`, malformed JSON, non-zero exit, timeout, and oversize output. |
| AC6 | Launcher tests cover bounded stdout/stderr handling, null stdin, timeout, stdout and stderr oversize refusal, child cleanup, and leaked stdout/stderr handles that would otherwise outlive the self-check child; integration status checks also time out a hanging managed launcher. |
| AC7 | Direct MCP no-env self-check reports `defaultDbMode:"replica"`; repo launcher `--check` reports final `replica`; synthetic packaged app-bundle manifest reports final `live`; launcher/status validation reject repo-binaries manifests that request `live`. |
| AC8 | Fake/stale sidecar cases are refused by manifest/provenance/hash tests before execution. No real production DB was opened for proof. |
| AC9 | Current real sidecar passed `--self-check-json` before server startup with `dbOpened:false`. |
| AC10 | Added `.docs/operations/safe-mcp-binary-setup.md`; release checklist now calls out real sidecar/provenance build before packaging. |
| AC11 | Focused L2 remediation tests, build proofs, fast gates, and the final full Rust gate passed, listed below. |

## Real Binary And Provenance Proof

Command:

```bash
bash src-tauri/scripts/build-mcp.sh
```

Result: passed. The script built release sidecars and wrote:

- `src-tauri/binaries/dailyos-mcp-aarch64-apple-darwin`
- `src-tauri/binaries/dailyos-mcp-launcher-aarch64-apple-darwin`
- `src-tauri/binaries/dailyos-mcp-bundle-aarch64-apple-darwin.provenance.json`

Provenance summary:

- `generatedAt`: `2026-06-03T00:38:34Z`
- `stub`: `false`
- `appBuildSha`: `2584b718b8cb2227756c6ffe4efe44577d481048`
- server SHA-256: `8f86fe3bab9fe4f02aac49af382c0a896f02a091778192b759b7e1ec6dc2cdfa`
- launcher SHA-256: `8995c72c752ad962a50f4502e5aa9c6ff00447959a8f48a8aaf90f8f51ebac9b`

L2 remediation rerun:

```bash
bash src-tauri/scripts/build-mcp.sh
```

Result: passed on 2026-06-04 after `build-mcp.sh` was changed to preserve existing
non-stub provenance while Cargo runs, then publish real provenance through a
same-directory temp file and atomic rename after build, copy, chmod, and hashing
succeed. The final rerun wrote:

- `generatedAt`: `2026-06-04T14:46:51Z`
- `stub`: `false`
- `appBuildSha`: `ab81de9c7b30f591c450e714bcb353dfe70a40ea`
- server SHA-256: `597f73beeff4438b91ebb983d4add96ce45bd7026b903b4f54318843ab4056da`
- launcher SHA-256: `ccb64757ec6137142d66882c3ee6e8d0313867966b15c3758315405823361fad`

CI stub path proof:

```bash
bash src-tauri/scripts/build-mcp.sh --stub
```

Result: passed on 2026-06-04. With existing non-stub provenance present, the script
preserved `dailyos-mcp-bundle-aarch64-apple-darwin.provenance.json` byte-for-byte
instead of downgrading it to `stub: true`:

- before SHA-256: `74e2a681ba0316de0b83415eab534795ea653a7e4584ef4e8ccdfeeb24abad11`
- after SHA-256: `74e2a681ba0316de0b83415eab534795ea653a7e4584ef4e8ccdfeeb24abad11`

Final L2 runtime proof after the symlink-publisher, startup-refresh, and
publish-race fixes:

```bash
bash -n src-tauri/scripts/build-mcp.sh
bash src-tauri/scripts/build-mcp.sh
readlink src-tauri/binaries/.dailyos-mcp-current-aarch64-apple-darwin
jq -r '[.stub, .generatedAt, .appBuildSha, (.sidecars[] | select(.name=="dailyos-mcp") | .sha256), (.sidecars[] | select(.name=="dailyos-mcp-launcher") | .sha256)] | @tsv' src-tauri/binaries/dailyos-mcp-bundle-aarch64-apple-darwin.provenance.json
shasum -a 256 src-tauri/binaries/dailyos-mcp-aarch64-apple-darwin src-tauri/binaries/dailyos-mcp-launcher-aarch64-apple-darwin
```

Result: passed on 2026-06-05. This specifically proves the macOS
symlink-to-directory replacement regression is fixed: current points to
`.dailyos-mcp-generations/aarch64-apple-darwin/20260605T062444Z-release-24939`.
The publisher now replaces `.dailyos-mcp-current-*` with Python `os.replace`
instead of `mv -f`, and clears `GENERATION_DIR` immediately after successful
activation so the cleanup trap cannot delete the active generation if a later
fsync or lock-release step fails.

Final provenance summary:

- `generatedAt`: `2026-06-05T06:24:44Z`
- `stub`: `false`
- `appBuildSha`: `ab81de9c7b30f591c450e714bcb353dfe70a40ea+dirty.74d024cd7642`
- server SHA-256: `9ada11437d42078080b982cd7fa84b7c936a1cc84fe892ca7d0bc518ed4acbbf`
- launcher SHA-256: `ca1e573403142a76dfaf5eba0624ad12162556148bea2690854a58e43206d13d`

Clean-checkout CI still gets stub binaries and stub provenance when the provenance
file is absent; local reruns no longer break an existing managed repo-binaries
manifest by replacing non-stub provenance with stub metadata.

Post-signing L2 follow-up:

- External adversarial review found that packaged app-bundle sidecars can be
  code-signed after `build-mcp.sh` records pre-sign hashes.
- Repo-binaries remain strict: manifest expected hashes must equal provenance
  hashes.
- App-bundle manifests now use the final measured launcher and sidecar hashes
  from `DailyOS.app/Contents/MacOS`, while still validating source kind, build
  SHA, target triple, provenance presence, executable metadata, and path shape.
- Tests simulate signing drift by writing app-bundle provenance for unsigned
  bytes, replacing the packaged files with signed bytes, and asserting the
  manifest uses the signed hashes while launcher verification still accepts the
  bundle.

Second L2 adversarial follow-up:

- External Codex review then found two P2 safety-boundary gaps: app-bundle
  configuration could bless a pre-config modified sidecar by recording its
  measured hash, and self-check timeout enforcement ended before stdout/stderr
  reader joins.
- App-bundle configuration now verifies the signed `.app` bundle with
  `codesign --verify --strict --deep` before recording final signed hashes; status
  validation rechecks signed app-bundle manifests before treating them as safe.
- The launcher now applies the same self-check deadline to child process exit and
  stdout/stderr reader completion, so leaked pipe handles produce
  `self_check_timeout` rather than hanging startup.

Third L2 adversarial follow-up:

- External Codex review then found that status validation reused a helper that
  repaired executable bits with `chmod`, so a read-only status check could mutate
  managed runtime files and report `Connected`.
- Runtime file hashing now observes executable metadata without changing it.
  Configure-time repo-binary resolution still uses an explicit repairing verifier
  before staging the managed launcher.
- Integration coverage now clears executable bits on both the managed launcher and
  MCP sidecar, verifies status reports unsafe, and asserts the files remain
  non-executable after status returns.

Fourth L2 subagent follow-up:

- Security subagent found a P0: the launcher itself did not enforce AppBundle
  authenticity, so a forged AppBundle manifest could choose `live` DB mode if it
  supplied matching hashes and self-check JSON. The launcher now requires
  AppBundle sidecar and provenance paths to canonicalize under the same `.app`
  root and verifies the signed bundle before running self-check or exec.
- Reliability subagent found unbounded `codesign` and self-check descendant leaks.
  `codesign` now runs under a timeout, launcher/status checks start children in
  their own process groups, and timeout handling kills the group before returning.
- Adversarial subagent found pid-only staging/backup names and direct config writes.
  Configure now holds a process-wide lock, uses per-invocation same-directory
  temp/backup names, and writes JSON through a temp file plus sync and rename.
- Integration coverage now runs two concurrent configure calls against one temp
  config root and verifies the resulting managed launcher/manifest remain valid.

Fifth L2 subagent follow-up:

- Security and adversarial reruns found that generic `codesign --verify` proves
  seal validity but not DailyOS origin. AppBundle verification now also inspects
  `codesign -dv --verbose=4` details and requires `Identifier=com.dailyos.desktop`
  plus the expected build-time `DAILYOS_APPLE_TEAM_ID`.
- `src-tauri/build.rs` derives `DAILYOS_APPLE_TEAM_ID` from an explicit
  environment variable or the release signing identity's trailing `(TEAMID)`.
  AppBundle runtime verification fails closed when the expected TeamIdentifier is
  absent.
- Launcher tests now reject AppBundles with the wrong TeamIdentifier or wrong
  bundle identifier before `live` DB mode can be selected.
- Reliability rerun found the outer launcher check timeout was shorter than
  AppBundle signature verification plus self-check. The status/configure launcher
  check timeout is now 15 seconds, covering both bounded phases with margin.

Sixth L2 subagent follow-up:

- Adversarial rerun found the configure transaction lock was process-local, so
  two DailyOS processes could interleave rollback and leave Claude config pointing
  at a missing launcher.
- Configure now opens `.dailyos/mcp/.configure.lock` with create/no-truncate,
  takes an exclusive OS file lock, and holds that guard from staging through
  managed launcher/manifest replacement and Claude config write.
- Integration coverage now proves a second file descriptor cannot acquire the
  configure lock while the first holder is alive.

Seventh L2 subagent follow-up:

- Reliability rerun found `.configure.lock` acquisition could block forever
  because the first implementation used a blocking OS lock.
- Configure now retries nonblocking exclusive `flock` acquisition on a bounded
  15-second deadline and reports a typed transaction-lock timeout instead of
  hanging the UI.
- Adversarial rerun found the managed launcher and manifest were still replaced
  in place, so a crash could leave a mismatched active pair. Configure now stages
  each successful runtime under an exclusive `runtime-<pid>-<time>-<seq>`
  generation directory and atomically points Claude config at that completed
  generation. Prior generations are preserved on failed and successful
  reconfigure.
- Directory creation, copied launcher files, temp JSON files, renames, and parent
  directories are synced before activation where local filesystems expose the
  required primitives.

Eighth L2 subagent follow-up:

- Reliability rerun found malformed Claude config JSON failed open by treating
  any parse error as "missing config"; configure now only creates `{}` when the
  file is absent. Invalid JSON, a top-level non-object, or a present non-object
  `mcpServers` value aborts before runtime staging and preserves the file.
- Security rerun found a mutable managed manifest could bless a tampered managed
  launcher. Status now derives the expected managed launcher hash from immutable
  provenance for repo binaries and from the signed AppBundle launcher sibling for
  packaged runtimes.
- Launcher checks now clear inherited environment, capture bounded stderr detail
  for refusal diagnostics, and truncate diagnostics on UTF-8 character
  boundaries.
- The launcher self-check and final sidecar exec both run under `env_clear()`;
  the final sidecar receives only the selected `DAILYOS_DB_MODE`.

Ninth L2 subagent follow-up:

- Security found that AppBundle validation still relied on generic
  `codesign --verify` plus parsed identity fields. Both configuration and
  launcher runtime verification now pass an explicit requirement to `codesign`:
  `anchor apple generic`, the DailyOS bundle identifier, and the expected
  Apple TeamIdentifier. The parsed `Identifier=` and `TeamIdentifier=` checks
  remain as a second fail-closed identity guard.
- Tests cover trusted-anchor requirement construction and invalid team-id
  rejection in both launcher and integration paths.

Tenth L2 subagent follow-up:

- Build proof found `mv -f "$CURRENT_TMP_LINK" "$CURRENT_LINK"` did not replace
  `.dailyos-mcp-current-*` on macOS when the destination was a symlink to a
  directory. It moved the temp symlink under the old generation instead,
  leaving stable paths pointed at stale artifacts.
- `build-mcp.sh` now uses Python `os.replace` to replace the current symlink
  itself. A final proof run demonstrated the active generation advanced.
- Reliability then found cleanup could still delete a newly active generation if
  a post-activation fsync failed before `GENERATION_DIR` was cleared. The script
  now clears `GENERATION_DIR` immediately after successful activation, before
  post-activation fsync and lock release.

Eleventh L2 subagent follow-up:

- Reliability found startup refresh would create a fresh runtime generation on
  every app launch even when the managed Claude Desktop config already pointed
  at a valid current launcher/manifest pair. Startup refresh now revalidates the
  existing managed config under the configure lock and returns without staging a
  generation when it is already current.
- The Tauri setup path now schedules a background startup refresh for existing
  DailyOS-managed Claude Desktop configs only. Missing, unmanaged, or
  env-overridden configs are preserved to keep user consent and manual
  overrides intact.
- Reliability also found staged runtime generations were cleaned only on
  launcher-validation failure. Configure now cleans staged generations on every
  pre-publish transaction failure, while preserving a generation if the Claude
  config was already renamed into place and only the parent directory fsync
  failed.

Twelfth L2 adversarial follow-up:

- Adversarial review found startup refresh could overwrite a concurrent Claude
  Desktop config edit because it wrote a stale snapshot after staging. Configure
  now rereads the config immediately before publish, validates `mcpServers`
  again, preserves unrelated concurrent edits, and aborts refresh if the
  `dailyos` entry changed between the locked recheck and final write.
- Adversarial review also found a parent fsync failure after config rename could
  report failure and then delete the runtime generation that the published
  config now referenced. Config writes now return whether the rename was
  published; cleanup runs only for failures before publication.
- Integration coverage now includes current-config no-op refresh,
  app-runtime-change refresh, unmanaged/env-overridden preservation, locked
  shape recheck, config publish parent-fsync failure preservation, concurrent
  config edit preservation, and staged-runtime cleanup on failed writes.

Final L2 review-cycle verdicts:

- Volta adversarial subagent: pass with non-blocking notes; no blocking
  findings. Residual note was limited to same-user tools that ignore the
  DailyOS lock and race the final Claude config rename boundary.
- Averroes security subagent: pass with non-blocking notes; no trust-boundary
  findings. Verified managed-config refresh gating, AppBundle trust anchoring,
  repo-binaries replica enforcement, and launcher env ownership.
- Laplace reliability subagent: pass with non-blocking notes; no blocking
  findings. Residual notes were limited to slow validation contention and
  unreferenced generation leakage after process kill or power loss.
- External Codex read-only gate: pass. It inspected the current working-tree
  diff against `public/dev`, including the uncommitted W1 remediation, and
  reported `P1 FINDINGS: none` and `P2/NOTES: none`.

Real MCP self-check command:

```bash
src-tauri/binaries/dailyos-mcp-aarch64-apple-darwin --self-check-json
```

Observed payload, with the executable path redacted:

```json
{
  "buildSha": "ab81de9c7b30f591c450e714bcb353dfe70a40ea+dirty.74d024cd7642",
  "dbOpened": false,
  "defaultDbMode": "replica",
  "executablePath": "~/Documents/dailyos-repo/.worktrees/codex/v1.4.9-w1-dos846/src-tauri/binaries/dailyos-mcp-aarch64-apple-darwin",
  "guardEpoch": "dailyos-mcp-runtime-guard:v1",
  "runtimeContainsDbModeGuard": true
}
```

Repo-binaries launcher check:

```bash
src-tauri/binaries/dailyos-mcp-launcher-aarch64-apple-darwin --manifest /private/tmp/dailyos-dos846-repo-manifest-rebased.json --check
```

Result:

```json
{
  "finalServerDbMode": "replica",
  "guardEpoch": "dailyos-mcp-runtime-guard:v1",
  "sidecarPath": "~/Documents/dailyos-repo/.worktrees/codex/v1.4.9-w1-dos846/src-tauri/binaries/dailyos-mcp-aarch64-apple-darwin",
  "status": "ok"
}
```

Synthetic app-bundle launcher check:

```bash
/private/tmp/dailyos-dos846-appbundle-rebased/DailyOS.app/Contents/MacOS/dailyos-mcp-launcher --manifest /private/tmp/dailyos-dos846-appbundle-rebased/manifest.json --check
```

Result:

```json
{
  "finalServerDbMode": "live",
  "guardEpoch": "dailyos-mcp-runtime-guard:v1",
  "sidecarPath": "/private/tmp/dailyos-dos846-appbundle-rebased/DailyOS.app/Contents/MacOS/dailyos-mcp",
  "status": "ok"
}
```

Packaging diagnostic:

- `pnpm tauri build --debug --bundles app` reached bundling and exposed the packaged MCP sidecar/resource layout.
- The command failed on an existing unrelated missing `release_gate` external-bin copy, so it is not counted as a green gate.
- The partially bundled `DailyOS.app` still copied real MCP files, not symlinks:
  `Contents/MacOS/dailyos-mcp` had SHA-256
  `6a5f66452b023b50528965042d5facfeaea406ce5dbb0df304159480aabbea25`,
  `Contents/MacOS/dailyos-mcp-launcher` had SHA-256
  `fdbfcb9eb67629d70569c1619e5bd163bff5ecc4320f79c15b615e1ee762d650`,
  and `Contents/Resources/binaries/dailyos-mcp-bundle-aarch64-apple-darwin.provenance.json`
  was non-stub and matched those artifact hashes.
- That diagnostic also exposed a Tauri external-bin name collision for the launcher. The implementation now uses the internal Cargo target `dailyos-mcp-launcher-bin` and copies it to the external-bin name `dailyos-mcp-launcher-$TARGET_TRIPLE`; the synthetic app-bundle check above proves the corrected packaged launcher/source-kind path.

## Focused Validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml --bin dailyos-mcp-launcher-bin
```

Passed: 5 tests.

Final L2 remediation rerun:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --bin dailyos-mcp-launcher-bin
```

Passed: 24 tests, including duplicate-argument refusal, repo-binaries/live DB-mode
refusal, app-bundle signed-hash/provenance drift acceptance, leaked self-check
stdio timeout, forged unsigned app-bundle manifest refusal, app-bundle root
mismatch refusal, self-check env clearing, final sidecar env ownership,
AppBundle path injection refusal, duplicate codesign identity refusal,
trusted-anchor requirement construction and invalid team-id rejection,
`manifest_refusal_matrix_blocks_before_running`,
`wrong_source_kind_path_is_refused_before_running`, and `self_check_refusal_matrix`
with deterministic stdout and stderr oversize cases.

```bash
cargo test --manifest-path src-tauri/Cargo.toml services::integrations --lib
```

Passed: 6 tests.

Final L2 remediation rerun:

```bash
cargo test --manifest-path src-tauri/Cargo.toml services::integrations --lib
```

Passed: 39 tests, including legacy raw-command status coverage for `target/debug`,
`target/release`, and `.cargo/bin`, raw-command rewrites, duplicate manifest arg
and env override refusal, wrong-target provenance refusal, repo-binaries/live
DB-mode refusal, packaged app-bundle final signed hash recording, unsigned
app-bundle rejection when signature verification is enabled, rollback failure
reporting, unsafe managed-runtime status states, failed and hanging
launcher-check status, non-executable runtime refusal without chmod, concurrent
configure safety, failed reconfigure preservation of the previous managed
runtime generation, successful reconfigure activation without overwriting the
previous generation, bounded file-lock acquisition, malformed config
preservation, managed launcher tamper rejection, launcher-check diagnostic
capture, UTF-8-safe diagnostic truncation, trusted-anchor requirement
construction, invalid team-id rejection, startup refresh no-op for current
managed configs, startup refresh after app runtime changes, unmanaged/env
override preservation, locked managed-shape rechecks, config publish parent-fsync
failure preservation, concurrent config edit preservation, and staged-runtime
cleanup on failed writes.

```bash
git diff --check
```

Passed.

## Release Gates

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Passed.

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Passed after the signed-bundle, pipe-timeout, AppBundle signature, startup-refresh,
publish-race, and warning-cleanup follow-ups. The final full Rust run after all
Rust-source changes reported `3174 passed; 0 failed; 11 ignored` for the main
library suite, `24 passed` for launcher bin tests, exit code 0 for the remaining
binary and integration tests, and doc tests reported `1 passed; 0 failed; 1
ignored`.

```bash
pnpm tsc --noEmit
```

Passed after hydrating checked-in dependencies with:

```bash
pnpm install --frozen-lockfile --ignore-scripts
```

## Local Safety Notes

- Tests use injected config roots and temp manifests; they do not read or edit the real Claude Desktop config.
- Proof commands used temp files under `/private/tmp` for app-bundle and manifest checks.
- Shared paths in this proof are redacted to `~` where they would otherwise include a home directory.
