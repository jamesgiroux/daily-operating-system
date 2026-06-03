# DOS-846 Proof Bundle - Guarded Claude Desktop MCP Runtime

**Date:** 2026-06-03
**Branch:** `codex/v1.4.9-w1-dos846`
**Issue:** [DOS-846](https://linear.app/a8c/issue/DOS-846)
**Build SHA used by real sidecars:** `2584b718b8cb2227756c6ffe4efe44577d481048`

The sidecar build SHA above is the validation build commit. The branch was
subsequently amended only to update this proof bundle; source-bearing files are
unchanged from that validation build. Verify with
`git diff --name-only 2584b718b8cb2227756c6ffe4efe44577d481048 HEAD`, which should
show only this proof file.

## Scope

DOS-846 closes the stale Claude Desktop MCP binary gap by moving Claude config to an app-managed launcher, verifying bundled build provenance before execution, running a bounded no-DB self-check, and pinning explicit DB mode for final MCP startup.

This work does not add a schema migration, MCP tool, claim field, or user-facing intelligence surface.

## Acceptance Evidence

| AC | Evidence |
| --- | --- |
| AC1 | `configure_claude_desktop()` now writes `mcpServers.dailyos.command` to the app-managed launcher with `--manifest`; focused tests verify raw `target/*` config is rewritten. |
| AC2 | `get_claude_desktop_status()` now marks raw commands, missing launchers, stale manifests, wrong hashes, and failed launcher checks as unsafe. |
| AC3 | `build-mcp.sh`, `Cargo.toml`, and `tauri.conf.json` build/package `dailyos-mcp`, `dailyos-mcp-launcher`, and provenance JSON. Synthetic app-bundle proof verifies packaged source-kind path semantics. |
| AC4 | Launcher tests cover missing provenance, wrong sidecar hash, and manifest/provenance mismatch before spawn. Integration tests reject stub provenance and stale manifests. |
| AC5 | Launcher validates `--self-check-json` guard epoch, build SHA, no-env `replica` default, guard presence, and `dbOpened:false`. |
| AC6 | Launcher test `hanging_self_check_times_out` covers bounded stdout/stderr handling, null stdin, timeout, and child cleanup. |
| AC7 | Direct MCP no-env self-check reports `defaultDbMode:"replica"`; repo launcher `--check` reports final `replica`; synthetic packaged app-bundle manifest reports final `live`. |
| AC8 | Fake/stale sidecar cases are refused by manifest/provenance/hash tests before execution. No real production DB was opened for proof. |
| AC9 | Current real sidecar passed `--self-check-json` before server startup with `dbOpened:false`. |
| AC10 | Added `.docs/operations/safe-mcp-binary-setup.md`; release checklist now calls out real sidecar/provenance build before packaging. |
| AC11 | Focused tests plus full gates passed, listed below. |

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

Real MCP self-check command:

```bash
src-tauri/binaries/dailyos-mcp-aarch64-apple-darwin --self-check-json
```

Observed payload, with the executable path redacted:

```json
{
  "buildSha": "2584b718b8cb2227756c6ffe4efe44577d481048",
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
- That diagnostic also exposed a Tauri external-bin name collision for the launcher. The implementation now uses the internal Cargo target `dailyos-mcp-launcher-bin` and copies it to the external-bin name `dailyos-mcp-launcher-$TARGET_TRIPLE`; the synthetic app-bundle check above proves the corrected packaged launcher/source-kind path.

## Focused Validation

```bash
cargo test --manifest-path src-tauri/Cargo.toml --bin dailyos-mcp-launcher-bin
```

Passed: 5 tests.

```bash
cargo test --manifest-path src-tauri/Cargo.toml services::integrations --lib
```

Passed: 6 tests.

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

Passed. The main library suite reported `3141 passed; 0 failed; 11 ignored`; launcher bin tests reported `5 passed`; the remaining integration and doc tests completed with exit code 0.

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
