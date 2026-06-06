# DOS-846 L0 Packet - Guarded Claude Desktop MCP Runtime

**Version:** v1.4.9 - W1 storage reset and safety close-out
**Issue:** [DOS-846](https://linear.app/a8c/issue/DOS-846)
**Author date:** 2026-06-02
**Tier:** Tier 3 markdown-only. **Scope tier:** Standard + security add-on.
**Branch/worktree:** `codex/v1.4.9-w1-dos846` at repo-relative `.worktrees/codex/v1.4.9-w1-dos846`

---

## §0 Origination, Scope, and Topology

- **Origination class:** Debug-driven. A stale pre-DB-mode-guard `dailyos-mcp` binary launched by Claude Desktop opened and migrated the live DB. DOS-820/821/822 closed guarded DB open paths for current code; DOS-846 closes the separate "what binary did Claude launch?" gap.
- **Scope tier:** Standard implementation with a security add-on. The packet changes MCP runtime selection, launcher/config generation, startup DB-mode defaulting, and proof docs. It does not add a schema migration, new MCP tools, new auth semantics, or claim/runtime intelligence behavior.
- **Trust topology:** Local-to-local single-user, with the existing v1.4.9 MCP carve-out preserved. `Confidential`/`UserOnly` egress policy is unchanged. The risk here is local dev/runtime safety, not remote adversary authorization.
- **Migration slots:** none.

### §0.1 Symptom-To-Failure Trace

1. Claude Desktop launches `mcpServers.dailyos.command` from `claude_desktop_config.json` without asking the app which build is current.
2. The current integration service still resolves candidate binaries from `~/.cargo/bin/dailyos-mcp` and raw `target/debug` / `target/release` paths (`src-tauri/src/services/integrations.rs:229-269`).
3. A stale binary built before the DB-mode guard has no `ProdOpenDenied` / replica-path guard code. Environment pins or current-source tests cannot protect a binary that simply does not contain the guard.
4. The current `dailyos-mcp` entrypoint calls `resolve_and_set_db_mode_from_process()` (`src-tauri/src/mcp/main.rs:1678-1680`). That helper defaults debug builds to `Replica` but release builds to `Live` when no explicit mode is present (`src-tauri/src/db/core.rs:156-164`).
5. Therefore a stale or wrong-profile MCP binary can open the live DB independently of the app, run migrations, and make the installed app unable to open the DB.

Rejected fixes:

- **Only set `DAILYOS_DB_MODE=replica` in Claude config.** This helps current binaries but does not stop a pre-guard binary that ignores the environment.
- **Only rebuild `target/debug/dailyos-mcp`.** This fixes one machine once. It leaves the configured path raw, mutable, and easy to stale again.
- **Only check for DB-mode guard inside `dailyos-mcp` startup.** A stale binary will not run that check.
- **Only execute the candidate with `--self-check-json`.** A pre-guard binary can hang as an MCP server or open the DB before it proves anything. Self-check is a second-phase semantic check, not the first safety boundary.
- **Install a second app-side DB service for MCP.** This is DOS-758/DOS-833 territory and does not solve the stale-binary selection problem.

---

## §1 Substrate Audit

Current ground truth on `dev`:

- `dailyos-mcp` is a Cargo bin gated by `required-features = ["mcp"]` (`src-tauri/Cargo.toml:139-142`).
- Tauri bundles the sidecar through `externalBin = ["binaries/dailyos-mcp"]` (`src-tauri/tauri.conf.json:33`).
- `src-tauri/scripts/build-mcp.sh` already builds the release sidecar with `--features mcp --bin dailyos-mcp` and copies it to `src-tauri/binaries/dailyos-mcp-$TARGET_TRIPLE`.
- `src-tauri/build.rs` already emits `BUILD_GIT_SHA` from `DAILYOS_BUILD_SHA`, `GITHUB_SHA`, or `git rev-parse HEAD` (`src-tauri/build.rs:25-51`). This can be reused to bind the app and sidecar to the same source revision.
- `ActionDb` has a structural prod-open deny at every guarded open chokepoint (`src-tauri/src/db/core.rs:235-254`, `:532-537`, `:679-684`, `:728-734`, `:800-812`), with unit tests for Replica/Mock denial (`src-tauri/src/db/core.rs:1211-1280`).
- CI already enforces that new direct file-backed SQLite opens stay behind known guarded chokepoints (`src-tauri/scripts/check_db_open_guard_allowlist.sh`, wired in `.github/workflows/lint-frontend.yml:109-110`).
- The integration service is the correct write boundary for Claude Desktop config (`src-tauri/src/services/integrations.rs`), and it already routes through `ServiceContext::check_mutation_allowed()` before editing the config.

Substrate that must not be reinvented:

- Reuse DB-mode guard and `ProdOpenDenied`; do not add a parallel "prod path" flag.
- Reuse `BUILD_GIT_SHA`; do not create a second version source.
- Reuse `build-mcp.sh`/Tauri `externalBin`; do not invent a second MCP build pipeline.
- Reuse `services::integrations`; do not add a command-layer config writer.

---

## §2 Chosen Architecture

DOS-846 ships a stable launcher contract for Claude Desktop:

1. **Claude config points to an app-managed compiled launcher, and legacy raw configs fail closed.**
   - The `mcpServers.dailyos.command` path becomes an app-managed launcher under DailyOS-owned local app data, for example `~/.dailyos/mcp/dailyos-mcp-launcher`.
   - The launcher is a compiled Rust bin, `dailyos-mcp-launcher`, not a generated shell script. It owns manifest parsing, canonical path checks, SHA-256 verification, bounded child-process self-check, child cleanup, and final server exec. It must not link to DB open/migration code or open any DB path.
   - `build-mcp.sh` builds both `dailyos-mcp` and `dailyos-mcp-launcher`, then copies both into `src-tauri/binaries/` as target-suffixed Tauri sidecars.
   - Tauri bundles both sidecars through `externalBin`, for example `["binaries/dailyos-mcp", "binaries/dailyos-mcp-launcher"]`.
   - `configure_claude_desktop()` copies the current verified launcher sidecar into `~/.dailyos/mcp/dailyos-mcp-launcher` and writes Claude config to that stable app-managed path with an explicit `--manifest ~/.dailyos/mcp/dailyos-mcp-manifest.json` argument.
   - `configure_claude_desktop()` rewrites any existing `mcpServers.dailyos` entry that points at raw `target/debug`, raw `target/release`, `~/.cargo/bin`, a missing launcher, or a launcher whose manifest no longer validates.
   - `get_claude_desktop_status()` must classify those legacy/direct command paths as unsafe, not connected. The user-visible status should require reconfiguration before Claude Desktop is considered safe.
   - Tests must inject temp Claude config, home/app-data roots, and candidate sidecar paths. No test may touch the real `~/Library/Application Support/Claude/claude_desktop_config.json`.

2. **The app writes a manifest from bundled build provenance; the launcher verifies before executing.**
   - `build-mcp.sh` emits build provenance alongside the sidecars, for example `dailyos-mcp-bundle-$TARGET_TRIPLE.provenance.json`, with guard epoch, app/sidecar `BUILD_GIT_SHA`, target triple, and expected SHA-256 plus filename for both `dailyos-mcp` and `dailyos-mcp-launcher`. The app must not establish trust by hashing an arbitrary candidate and treating that measured hash as authoritative.
   - Tauri bundles the provenance JSON as a resource, for example by adding `binaries/*.provenance.json` to `bundle.resources`. In the installed app, the resolver reads it from the packaged resources location (`DailyOS.app/Contents/Resources/...`) or equivalent Tauri resource path.
   - `build-mcp.sh --stub` may create clearly marked stub binaries/provenance for local dependency validation, but release/package validation must reject `stub: true`. The launcher also rejects zero-byte/stub sidecars before spawn.
   - The app-managed launcher manifest copies trusted expected values from bundled/dev build provenance after verifying app build SHA, launcher build SHA, sidecar build SHA, guard epoch, target triple, filenames/source kind, launcher SHA-256, and sidecar SHA-256.
   - The app-managed launcher manifest records at least: manifest schema version, guard epoch, app `BUILD_GIT_SHA`, launcher `BUILD_GIT_SHA`, sidecar `BUILD_GIT_SHA`, selected source kind (`app_bundle` or `repo_binaries`), launcher path, sidecar path, expected launcher SHA-256, expected sidecar SHA-256, final server DB mode, and generated timestamp.
   - Candidate sidecars are limited to:
     - packaged app sidecar and launcher derived from the running Tauri app bundle path under `DailyOS.app/Contents/MacOS/`, with expected filenames sourced from bundled build provenance;
     - dev sidecar and launcher at `src-tauri/binaries/dailyos-mcp-$TARGET_TRIPLE` and `src-tauri/binaries/dailyos-mcp-launcher-$TARGET_TRIPLE`, produced by `src-tauri/scripts/build-mcp.sh`, with matching dev provenance.
   - Raw `target/debug`, raw `target/release`, and `~/.cargo/bin` are removed from resolver candidates.
   - The launcher canonicalizes the manifest path, its own path, and sidecar path; verifies launcher/sidecar are inside the recorded allowed root/source kind; rejects missing/non-executable/zero-byte/stub files; computes SHA-256; and compares launcher plus sidecar hashes with expected provenance hashes before spawning anything.
   - A moved app, stale launcher, edited launcher, edited sidecar, zero-byte Tauri stub, missing bundled provenance, or wrong hash fails closed before the candidate MCP binary is executed.

3. **The launcher runs structured self-check only after provenance passes.**
   - After the non-executing path/hash check passes, the launcher runs the candidate sidecar with `--self-check-json` using no `DAILYOS_DB_MODE` env and no DB-mode CLI flags.
   - Self-check process handling is explicit: stdin closed or null, stdout captured with a small bounded limit, stderr captured or discarded with a bounded limit, timeout at a short fixed interval, and child kill on timeout.
   - The self-check reports at least: guard epoch, build SHA, resolved no-env default DB mode, executable path, whether the runtime contains the MCP DB-mode guard contract, and that no DB was opened for self-check.
   - Missing, malformed, non-zero, timed-out, wrong-build, wrong-guard, guard-absent, or unexpected-default self-check output means fail closed.
   - Self-check must assert no-env/default `Replica`. The final server exec receives the explicit packaged Live or dev Replica env only after self-check succeeds.
   - The stale/pre-guard binary case is safe because a stale file cannot pass sidecar-provenance build SHA/guard epoch/hash verification; the self-check is defense in depth for a verified current sidecar.

4. **Generated Claude Desktop config pins an explicit DB mode without weakening MCP as a product head.**
   - The generated config and launcher never rely on release-build defaulting to Live.
   - For packaged/installed app sidecars, the launcher pins `DAILYOS_DB_MODE=live` after provenance and self-check pass, preserving MCP as a co-equal product head for feedback/write flows under the existing service and ability contracts.
   - For repo/dev sidecars, the launcher pins `DAILYOS_DB_MODE=replica` by default unless a deliberate developer opt-in path explicitly requests Live.
   - Direct `dailyos-mcp` invocation with no explicit DB mode defaults to Replica, including release builds. Explicit `--live` or `DAILYOS_DB_MODE=live` remains possible for deliberate diagnostics and app-generated production launcher use.

5. **The self-check contract is embedded in the sidecar deliberately.**
   - Add a small library-owned MCP runtime guard module with constants such as `dailyos-mcp-runtime-guard:v1`, `dailyos-mcp-db-mode-default:replica`, and `dailyos-build-sha:<BUILD_GIT_SHA>`.
   - `src-tauri/src/mcp/main.rs` handles `--self-check-json` before server startup and returns the structured payload without opening the DB.
   - `build-mcp.sh` may perform a marker sanity check after build as defense in depth, but runtime correctness relies on manifest/hash verification plus structured self-check output rather than brittle release-binary symbol scanning.

6. **Safe setup and proof output are documented.**
   - Add a repo doc for building and registering a guarded MCP sidecar.
   - The doc must say `--features mcp` is required and that Claude Desktop should be configured through the app/service, not by hand-pointing to `target/debug`.
   - Shared proof, Linear comments, and PR notes must redact home-directory paths as `~` or `~/.dailyos/...`. Full absolute paths may remain in local-only diagnostics when needed for setup.

Out of scope:

- DOS-833 auth right-sizing.
- DOS-758 request-scoped handler DB context.
- SQLCipher removal (DOS-831).
- Editing any real local Claude Desktop config as part of tests.
- Using real customer or production data in proof.
- Destructive stale-binary cleanup. The work may refuse or overwrite the app-managed launcher/config entry, but it must not delete user files or raw binaries as a hidden side effect.

---

## §3 Acceptance Criteria

- **AC1.** `configure_claude_desktop()` writes `mcpServers.dailyos.command` to the app-managed launcher and rewrites existing raw `target/debug`, raw `target/release`, or `~/.cargo/bin` entries.
- **AC2.** `get_claude_desktop_status()` marks direct/raw commands, missing launchers, stale manifests, wrong hashes, and failed self-checks as unsafe rather than connected.
- **AC3.** `build-mcp.sh`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` define a buildable packaged layout for `dailyos-mcp`, `dailyos-mcp-launcher`, and bundled build provenance. Installed-app layout tests/proof verify the provenance JSON is available from the packaged resources path.
- **AC4.** The launcher refuses to execute any sidecar until a non-executing build-provenance plus manifest/path/hash check succeeds. A missing provenance file, stub provenance, mismatched launcher/sidecar build SHA, moved app, missing launcher/sidecar, wrong path, wrong hash, or zero-byte Tauri stub is refused before spawn.
- **AC5.** After provenance passes, the launcher refuses a sidecar whose `--self-check-json` is missing, malformed, non-zero, timed out, guard-absent, wrong guard epoch, wrong build SHA, or no-env DB-mode default other than Replica.
- **AC6.** The launcher self-check subprocess has bounded stdout/stderr, closed/null stdin, no DB-mode env/flags, a short timeout, and child kill on timeout. A hanging fake binary is covered by tests.
- **AC7.** Generated config never relies on implicit release Live defaulting. Packaged app launcher final exec pins explicit Live after verification; repo/dev launcher final exec pins explicit Replica by default; direct no-env MCP defaults to Replica.
- **AC8.** A deliberately stale/fake pre-guard binary in the launcher target path is refused before any DB open can occur. The proof uses temp files/fake binaries only; no real prod DB is opened or mutated.
- **AC9.** A current guarded sidecar passes the real `dailyos-mcp --self-check-json` path before server startup, with DB-mode env absent, and reports guard epoch, build SHA, no-env Replica default, and no DB open.
- **AC10.** Documentation explains safe MCP setup, required build command, legacy-config remediation, and redacted proof output.
- **AC11.** `cargo clippy -- -D warnings`, `cargo test`, and `pnpm tsc --noEmit` pass.

---

## §4 Implementation Surface

Likely files:

- `src-tauri/src/mcp/main.rs` - MCP-specific DB-mode default and marker reference.
- `src-tauri/src/mcp/launcher.rs` or equivalent - compiled `dailyos-mcp-launcher` bin with no DB open/migration path.
- `src-tauri/Cargo.toml` - add the `dailyos-mcp-launcher` bin entry.
- `src-tauri/src/db/core.rs` - reusable resolver helper for explicit process DB mode with a caller-supplied default.
- `src-tauri/src/services/integrations.rs` - launcher generation, manifest generation, verified sidecar resolution, legacy-config status/remediation, config shape, injectable filesystem roots, redaction helpers, and tests.
- `src-tauri/scripts/build-mcp.sh` - build/copy both binaries, emit bundle provenance, create stub artifacts only when requested, and fail real sidecar builds when produced binaries lack required markers.
- `src-tauri/tauri.conf.json` - bundle the launcher sidecar and provenance resource.
- `.docs/RELEASE-CHECKLIST.md` - ensure the MCP build/provenance step remains explicit before Tauri packaging.
- `src-tauri/src/doctor.rs` - optional `doctor mcp` diagnostic if implementation needs operator-visible proof of the configured command, env pin, and self-check result.
- `.docs/operations/safe-mcp-binary-setup.md` or equivalent - safe setup doc.
- Focused tests under existing module test locations.

Avoid:

- No new schema or migrations.
- No broad MCP auth refactor.
- No new DB open chokepoint.
- No hand-edit of `~/Library/Application Support/Claude/claude_desktop_config.json` in tests.

---

## §5 Test and Proof Plan

Focused tests:

- Unit-test the sidecar resolver with temp app/dev layouts:
  - resolves the actual Tauri packaged sidecar and launcher convention under `DailyOS.app/Contents/MacOS/`;
  - resolves bundled build provenance from packaged resources;
  - prefers verified bundle/dev sidecar plus launcher;
  - rejects stubs;
  - ignores `target/debug`, `target/release`, and `~/.cargo/bin`.
- Unit-test legacy config status/remediation with injected temp config roots:
  - existing direct `target/*` and `~/.cargo/bin` commands are unsafe;
  - `configure_claude_desktop()` rewrites the entry to the launcher;
  - a missing launcher, stale manifest, or wrong sidecar hash is unsafe until reconfigured;
  - no test reads or writes the real Claude Desktop config path.
- Unit-test launcher generation:
  - command path is launcher;
  - args include `--manifest <app-managed-manifest-path>`;
  - build provenance is required and must match app build SHA, guard epoch, target triple, allowed filenames/source kind, and actual launcher/sidecar SHA-256;
  - manifest includes launcher path, sidecar path, expected launcher SHA-256, expected sidecar SHA-256, guard epoch, app build SHA, launcher build SHA, sidecar build SHA, sidecar kind, and final server DB mode;
  - copied app-managed launcher matches the expected launcher SHA-256 and receives the manifest path through Claude config args.
- Shell-execute generated launcher against temp fake executables:
  - fake stale sidecar with the wrong hash is refused before it can run;
  - fake sidecar with matching trusted fixture provenance but no `--self-check-json` support exits non-zero in an isolated second-phase test fixture;
  - fake hanging sidecar is killed on timeout;
  - fake sidecar with valid self-check/build SHA records no DB-mode env during self-check and explicit DB-mode env only during final exec simulation.
- Unit-test MCP DB-mode resolver:
  - no explicit mode -> Replica for MCP default;
  - packaged final exec env -> Live only after launcher verification and self-check;
  - repo/dev final exec env -> Replica by default;
  - explicit `--live` or `DAILYOS_DB_MODE=live` still resolves Live only when deliberately provided.
- Unit-test self-check:
  - returns JSON without opening DB;
  - includes guard epoch, build SHA, and default DB mode;
  - reports no PII or path beyond the executable/config paths needed for local diagnosis.
- Add a real binary proof path for `dailyos-mcp --self-check-json`:
  - the built current binary returns before server startup;
  - no DB path is opened for self-check;
  - the self-check process runs with DB-mode env absent;
  - the payload matches the expected guard epoch/build SHA/no-env Replica default.

Full gates:

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tsc --noEmit
```

Proof bundle:

- Test output for stale-binary refusal.
- Test output for hanging-binary timeout and child cleanup.
- Test output for current real self-check launcher success.
- Evidence that packaged app layout exposes both launcher and MCP sidecars plus build provenance.
- Evidence that no configured command path contains `/target/debug/`, `/target/release/`, or `/.cargo/bin/`.
- Evidence that no test opened or mutated real `~/.dailyos/dailyos.db`.
- Shared evidence redacts home-directory paths as `~` or `~/.dailyos/...`.

---

## §6 Intelligence Loop Integration Check

This is substrate/runtime safety work only.

1. **Claim model:** No new claim/table/field/user-visible intelligence output.
2. **Provenance and trust:** No trust-band semantics change. Existing MCP provenance/sensitivity behavior remains untouched.
3. **Signals and invalidation:** No claim/signal propagation change.
4. **Runtime and surfaces:** Tauri integration config and MCP runtime startup are the consuming surfaces. Packaged Claude Desktop MCP remains a guarded Live product head after launcher verification; dev/direct no-env invocation defaults to Replica so stale/manual paths fail safer. No MCP ability, provenance, or service-boundary semantics change.
5. **Feedback loop:** No user correction/corroboration/dismissal flow change.

---

## §7 Reviewer Dispatch (L0)

- **Default:** `/codex challenge`.
- **Routed planning reviewer:** `ce-feasibility-reviewer` because the plan depends on build/runtime path realism.
- **Amendment-3 add-on:** `ce-security-lens-reviewer` because the change touches MCP, filesystem config, and prod/open guard posture.
- **K-in:** `ce-learnings-researcher` over `docs/solutions/` and `.docs/decisions/`.

Unanimous approval is required before L1 code starts.

---

## §8 K-In Findings

Cycle-1 K-in review completed and these findings are folded into the packet:

- `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md` - this incident is adjacent to the same recurring DB-open/connection discipline class. The plan must not add new writer/open paths.
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md` - static/grep/string checks are useful but insufficient; the launcher behavior test and structured self-check carry the proof.
- ADR-0071 - forward-compatibility is already a binary/schema safety precedent; DOS-846 applies the same fail-closed instinct before MCP startup rather than after a DB open.
- ADR-0092 - current SQLCipher/key/open sequence remains untouched; MCP read-only open still needs a key until DOS-831 changes that. DOS-846 must not argue from FileVault-only storage.
- ADR-0101 - Claude config mutation stays in `services::integrations`.
- ADR-0104 - MCP production runs still operate as `ExecutionMode::Live`; dev safety is explicit DB target selection, not a new execution mode.
- ADR-0108/0120 - guard diagnostics/logs carry shape only: versions, hashes, typed failure kinds, and paths needed for setup; no user content, prompts, responses, subject names, or raw provenance.
- ADR-0128 - MCP is a product head over the substrate; runtime safety must preserve the headless surface rather than treating it as disposable dev plumbing.
- ADR-0133/0134 - do not solve stale-binary safety by multiplying DB connections/pools or changing reader/writer sizing.

---

## §9 L0 Review Response Log

Cycle-1 verdict was `CHANGES-REQUIRED`. The packet was revised before L1 to close the blocking findings:

- **Legacy config bypass:** status now must flag direct/raw Claude commands as unsafe, and configure must rewrite them to the launcher.
- **Pre-exec safety:** launcher now performs non-executing manifest/path/hash verification before spawning any candidate binary.
- **Hanging/stale process behavior:** self-check has bounded IO, timeout, and child-kill requirements plus fake hanging-binary tests.
- **Product MCP semantics:** packaged Claude Desktop MCP pins explicit Live only after verification; dev/direct paths default Replica so this does not silently move product feedback writes to a replica DB.
- **Real packaged/dev locator:** sidecar resolution is constrained to actual Tauri app sidecar layout or `src-tauri/binaries/dailyos-mcp-$TARGET_TRIPLE`; raw target/cargo paths are excluded.
- **Test seams and privacy:** config/root/path helpers must be injectable for temp-dir tests, and shared proof redacts home-directory paths.

Cycle-2 verdict was `CHANGES-REQUIRED` from `/codex challenge` and feasibility, with security approved. The packet was revised again before L1:

- **Packaged provenance:** build provenance is explicitly bundled as a Tauri resource and tested in installed-app layout; `tauri.conf.json` and `build-mcp.sh` are in scope.
- **Launcher artifact:** launcher is now a compiled Rust `dailyos-mcp-launcher` sidecar, bundled and copied to the stable app-managed command path, not an unspecified generated launcher.
- **Self-check env isolation:** `--self-check-json` runs with no DB-mode env/flags and must prove no-env Replica default; explicit Live/Replica env is applied only to final server exec.

Cycle-3 verdict is `APPROVE`:

- **/codex challenge:** approve. The read-only sandbox prevented a nested external Codex initialization inside the reviewer run, but the challenge pass reviewed the packet and returned no blocking findings.
- **ce-feasibility-reviewer:** approve, no blocking feasibility findings.
- **ce-security-lens-reviewer:** approve, no blocking security findings.

L0 is approved unanimously. L1 may start after this verdict is mirrored to Linear.

---

## §10 Definition of Done

- L0 packet approved unanimously.
- DOS-846 Linear ticket updated with L0 verdict and final scope.
- L1 implements the launcher, resolver, runtime default, docs, and focused tests.
- L2 passes before PR creation, including security review.
- PR links DOS-846, carries `L2-status: passed`, includes `security_auditor_invoked: true`, and targets `dev`.
